//! Shared tool surface for MCP clients and in-app OpenAI tool-calling.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::db::connection::Connection;
use crate::embeddings::EmbeddingHub;
use crate::error::{AppError, AppResult};
use crate::models::{
    InstallAnnRequest, ReindexRequest, SemanticSearchRequest, SqlRequest, ToolTrace,
};
use parking_lot::RwLock;

pub type SharedDb = Arc<RwLock<Option<Connection>>>;

pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "list_tables",
            "List all tables in the open MongrelDB database with row counts and index capabilities.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "describe_table",
            "Describe a table: columns, types, flags, and secondary indexes (Bitmap/PGM/FM/ANN/Sparse/MinHash).",
            json!({
                "type": "object",
                "properties": {
                    "table": { "type": "string", "description": "Table name" }
                },
                "required": ["table"],
                "additionalProperties": false
            }),
        ),
        tool(
            "database_overview",
            "High-level overview of the open database: path or server URL, connection mode, tables, ANN readiness.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "execute_sql",
            "Run a MongrelDB SQL statement (DataFusion). Prefer read-only SELECT for exploration.",
            json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string" },
                    "max_rows": { "type": "integer", "minimum": 1, "maximum": 10000 }
                },
                "required": ["sql"],
                "additionalProperties": false
            }),
        ),
        tool(
            "semantic_search",
            "Dense ANN semantic search using a query string (requires embedding column + ANN index).",
            json!({
                "type": "object",
                "properties": {
                    "table": { "type": "string" },
                    "embedding_column": { "type": "string", "default": "embedding" },
                    "query": { "type": "string" },
                    "k": { "type": "integer", "default": 5 },
                    "exact_rerank": { "type": "boolean", "default": true },
                    "min_score": { "type": "number", "description": "Cosine similarity floor; drops weak hits (exact path only)" },
                    "projection": { "type": "string" }
                },
                "required": ["table", "query"],
                "additionalProperties": false
            }),
        ),
        tool(
            "install_dense_ann",
            "Install ANN (HNSW) on a table (direct/local only). Default quantization is dense (full f32 cosine); pass binary_sign for legacy compact Hamming. Set rebuild=true to drop and recreate an existing ANN (e.g. BinarySign → Dense).",
            json!({
                "type": "object",
                "properties": {
                    "table": { "type": "string" },
                    "embedding_column": { "type": "string", "default": "embedding" },
                    "dimension": { "type": "integer", "default": 384 },
                    "source_text_column": { "type": "string" },
                    "provider_id": { "type": "string" },
                    "backfill_limit": { "type": "integer" },
                    "quantization": {
                        "type": "string",
                        "enum": ["dense", "binary_sign"],
                        "default": "dense",
                        "description": "dense = full f32 cosine ANN; binary_sign = legacy compact Hamming"
                    },
                    "rebuild": {
                        "type": "boolean",
                        "default": false,
                        "description": "Drop existing ANN on the embedding column and recreate with the requested options"
                    }
                },
                "required": ["table"],
                "additionalProperties": false
            }),
        ),
        tool(
            "reindex",
            "Run engine REINDEX maintenance (analyze + compact + GC). Omit table for the whole database; pass table for one table.",
            json!({
                "type": "object",
                "properties": {
                    "table": {
                        "type": "string",
                        "description": "Optional table name. Empty/omit = REINDEX entire database."
                    }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "constellation",
            "Return the schema constellation graph (tables, columns, indexes).",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "list_embedding_providers",
            "List embedding providers available in the viewer.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
    ]
}

fn tool(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": parameters,
    })
}

pub fn openai_tools() -> Vec<Value> {
    tool_definitions()
        .into_iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t["name"],
                    "description": t["description"],
                    "parameters": t["inputSchema"],
                }
            })
        })
        .collect()
}

pub struct ToolExecutor {
    pub db: SharedDb,
    pub embeddings: EmbeddingHub,
}

impl ToolExecutor {
    pub fn new(db: SharedDb, embeddings: EmbeddingHub) -> Self {
        Self { db, embeddings }
    }

    pub async fn call(&self, name: &str, arguments: Value) -> ToolTrace {
        match self.call_inner(name, arguments.clone()).await {
            Ok(result) => ToolTrace {
                name: name.into(),
                arguments,
                result,
                ok: true,
            },
            Err(e) => ToolTrace {
                name: name.into(),
                arguments,
                result: json!({ "error": e.to_string() }),
                ok: false,
            },
        }
    }

    async fn call_inner(&self, name: &str, arguments: Value) -> AppResult<Value> {
        match name {
            "list_tables" => {
                let guard = self.db.read();
                let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
                let overview = conn.overview()?;
                Ok(json!(overview.tables))
            }
            "describe_table" => {
                let table = arg_str(&arguments, "table")?;
                let guard = self.db.read();
                let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
                Ok(json!(conn.table_detail(&table)?))
            }
            "database_overview" => {
                let guard = self.db.read();
                let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
                let mut overview = conn.overview()?;
                overview.embedding_providers = self.embeddings.list_providers();
                Ok(json!(overview))
            }
            "execute_sql" => {
                let sql = arg_str(&arguments, "sql")?;
                let max_rows = arguments
                    .get("max_rows")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);
                let work = {
                    let guard = self.db.read();
                    let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
                    conn.sql_work(SqlRequest { sql, max_rows })?
                };
                Ok(json!(work.run().await?))
            }
            "semantic_search" => {
                let table = arg_str(&arguments, "table")?;
                let query = arg_str(&arguments, "query")?;
                let embedding_column = arguments
                    .get("embedding_column")
                    .and_then(|v| v.as_str())
                    .unwrap_or("embedding")
                    .to_string();
                let k = arguments
                    .get("k")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);
                let exact_rerank = arguments.get("exact_rerank").and_then(|v| v.as_bool());
                let min_score = arguments
                    .get("min_score")
                    .and_then(|v| v.as_f64())
                    .map(|n| n as f32);
                let projection = opt_str(&arguments, "projection");
                let result = crate::db::semantic_search_on_connection(
                    &self.db,
                    &self.embeddings,
                    SemanticSearchRequest {
                        table,
                        embedding_column,
                        query,
                        k,
                        provider_id: None,
                        projection,
                        exact_rerank,
                        min_score,
                    },
                )
                .await?;
                Ok(json!(result))
            }
            "install_dense_ann" => {
                let table = arg_str(&arguments, "table")?;
                let view = {
                    let guard = self.db.read();
                    let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
                    let direct = conn.as_direct()?;
                    crate::db::DbSession {
                        path: direct.path.clone(),
                        database: std::sync::Arc::clone(&direct.database),
                        session: std::sync::Arc::clone(&direct.session),
                        opened_at: direct.opened_at,
                        credentials_required: direct.credentials_required,
                    }
                };
                let result = crate::db::install_dense_ann(
                    &view,
                    &self.embeddings,
                    InstallAnnRequest {
                        table,
                        embedding_column: opt_str(&arguments, "embedding_column"),
                        dimension: arguments
                            .get("dimension")
                            .and_then(|v| v.as_u64())
                            .map(|n| n as u32),
                        source_text_column: opt_str(&arguments, "source_text_column"),
                        provider_id: opt_str(&arguments, "provider_id"),
                        index_name: None,
                        m: None,
                        ef_construction: None,
                        ef_search: None,
                        backfill_limit: arguments
                            .get("backfill_limit")
                            .and_then(|v| v.as_u64())
                            .map(|n| n as usize),
                        quantization: opt_str(&arguments, "quantization"),
                        rebuild: arguments.get("rebuild").and_then(|v| v.as_bool()),
                    },
                )
                .await?;
                Ok(json!(result))
            }
            "reindex" => {
                let table = opt_str(&arguments, "table");
                let direct = {
                    let guard = self.db.read();
                    let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
                    match conn {
                        Connection::Direct(d) => Some(crate::db::DbSession {
                            path: d.path.clone(),
                            database: std::sync::Arc::clone(&d.database),
                            session: std::sync::Arc::clone(&d.session),
                            opened_at: d.opened_at,
                            credentials_required: d.credentials_required,
                        }),
                        Connection::Server(_) => None,
                    }
                };
                if let Some(view) = direct {
                    let result = crate::db::reindex(
                        &view,
                        ReindexRequest { table },
                    )
                    .await?;
                    return Ok(json!(result));
                }
                // Server: issue REINDEX SQL over HTTP.
                let sql = match table.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    None => "REINDEX".to_string(),
                    Some(name) => {
                        if !name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                        {
                            return Err(AppError::msg(format!(
                                "invalid table name {name:?}"
                            )));
                        }
                        format!("REINDEX {name}")
                    }
                };
                let work = {
                    let guard = self.db.read();
                    let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
                    conn.sql_work(SqlRequest {
                        sql,
                        max_rows: Some(1),
                    })?
                };
                work.run().await?;
                Ok(json!({
                    "target": table.unwrap_or_else(|| "database".into()),
                    "message": "REINDEX completed",
                }))
            }
            "constellation" => {
                let guard = self.db.read();
                let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
                Ok(json!(conn.constellation()?))
            }
            "list_embedding_providers" => Ok(json!(self.embeddings.list_providers())),
            other => Err(AppError::Mcp(format!("unknown tool: {other}"))),
        }
    }
}

fn arg_str(args: &Value, key: &str) -> AppResult<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::msg(format!("missing string argument: {key}")))
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
