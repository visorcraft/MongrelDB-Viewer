//! Unified connection: exclusive direct (embedded) or multi-client server HTTP.

use std::sync::Arc;
use std::time::Instant;

use mongreldb_client::MongrelClient;
use mongreldb_query::MongrelSession;
use parking_lot::RwLock;

use crate::db::session::{DbSession, OpenMode};
use crate::error::{AppError, AppResult};
use crate::models::{
    ColumnInfo, ConstellationGraph, DatabaseOverview, GraphEdge, GraphNode, IndexInfo, IndexRadar,
    ServerOpenRequest, SqlRequest, SqlResult, TableDetail, TableSummary,
};

/// Active connection held by the viewer.
pub enum Connection {
    Direct(DbSession),
    Server(ServerSession),
}

pub struct ServerSession {
    pub url: String,
    pub client: MongrelClient,
    pub opened_at: chrono::DateTime<chrono::Utc>,
    pub health: String,
}

pub type SharedConnection = Arc<RwLock<Option<Connection>>>;

impl Connection {
    pub fn as_direct(&self) -> AppResult<&DbSession> {
        match self {
            Self::Direct(d) => Ok(d),
            Self::Server(_) => Err(AppError::msg(
                "This action needs a direct (local) connection. Disconnect and open a database folder, or run the SQL on the server instead.",
            )),
        }
    }

    pub fn open_direct(
        path: impl AsRef<std::path::Path>,
        username: Option<&str>,
        password: Option<&str>,
        passphrase: Option<&str>,
        mode: OpenMode,
    ) -> AppResult<Self> {
        Ok(Self::Direct(DbSession::open(
            path, username, password, passphrase, mode,
        )?))
    }

    pub fn create_demo(path: impl AsRef<std::path::Path>, with_ann: bool) -> AppResult<Self> {
        Ok(Self::Direct(DbSession::create_demo(path, with_ann)?))
    }

    pub fn open_server(req: &ServerOpenRequest) -> AppResult<Self> {
        let url = req.url.trim().trim_end_matches('/').to_string();
        if url.is_empty() {
            return Err(AppError::msg("Server URL is required (e.g. http://127.0.0.1:8453)"));
        }
        let mut builder = MongrelClient::builder(&url)
            .connect_timeout(std::time::Duration::from_secs(5))
            .request_timeout(std::time::Duration::from_secs(120));
        if let Some(token) = req.bearer_token.as_deref().filter(|t| !t.is_empty()) {
            builder = builder.bearer_token(token);
        }
        if let (Some(u), Some(p)) = (
            req.username.as_deref().filter(|s| !s.is_empty()),
            req.password.as_deref(),
        ) {
            builder = builder.basic_auth(u, p);
        }
        let client = builder
            .build()
            .map_err(|e| AppError::msg(format!("failed to create server client: {e}")))?;
        let health = client
            .health()
            .map_err(|e| AppError::msg(format!("cannot reach mongreldb-server at {url}: {e}")))?;
        // Prove we can list tables (auth / path errors surface here).
        let _ = client
            .list_tables()
            .map_err(|e| AppError::msg(format!("connected but list_tables failed: {e}")))?;
        Ok(Self::Server(ServerSession {
            url,
            client,
            opened_at: chrono::Utc::now(),
            health,
        }))
    }

    pub fn overview(&self) -> AppResult<DatabaseOverview> {
        match self {
            Self::Direct(d) => {
                let mut ov = crate::db::inspect::database_overview(d)?;
                ov.connection_mode = "direct".into();
                ov.display_label = d.path.display().to_string();
                Ok(ov)
            }
            Self::Server(s) => server_overview(s),
        }
    }

    pub fn table_detail(&self, name: &str) -> AppResult<TableDetail> {
        match self {
            Self::Direct(d) => crate::db::inspect::table_detail(d, name),
            Self::Server(s) => server_table_detail(s, name),
        }
    }

    pub fn constellation(&self) -> AppResult<ConstellationGraph> {
        match self {
            Self::Direct(d) => crate::db::inspect::build_constellation(d),
            Self::Server(s) => server_constellation(s),
        }
    }

    /// Prepare SQL work without holding locks across await.
    pub fn sql_work(&self, req: SqlRequest) -> AppResult<SqlWork> {
        match self {
            Self::Direct(d) => Ok(SqlWork::Direct {
                session: Arc::clone(&d.session),
                req,
            }),
            Self::Server(s) => Ok(SqlWork::Server {
                client: s.client.clone(),
                req,
            }),
        }
    }

    pub async fn run_sql(&self, req: SqlRequest) -> AppResult<SqlResult> {
        self.sql_work(req)?.run().await
    }
}

pub enum SqlWork {
    Direct {
        session: Arc<MongrelSession>,
        req: SqlRequest,
    },
    Server {
        client: MongrelClient,
        req: SqlRequest,
    },
}

impl SqlWork {
    pub async fn run(self) -> AppResult<SqlResult> {
        match self {
            Self::Direct { session, req } => crate::db::sql::run_sql_session(session, req).await,
            Self::Server { client, req } => {
                let sql = req.sql;
                let max_rows = req.max_rows.unwrap_or(500).clamp(1, 10_000);
                tokio::task::spawn_blocking(move || server_run_sql(&client, &sql, max_rows))
                    .await
                    .map_err(|e| AppError::msg(format!("sql task failed: {e}")))?
            }
        }
    }
}

fn server_overview(s: &ServerSession) -> AppResult<DatabaseOverview> {
    let names = s
        .client
        .list_tables()
        .map_err(|e| AppError::msg(format!("list_tables: {e}")))?;
    let mut tables = Vec::with_capacity(names.len());
    for name in &names {
        tables.push(server_table_summary(s, name)?);
    }
    tables.sort_by(|a, b| a.name.cmp(&b.name));
    let engine = mongreldb_core::build_info();
    let query = mongreldb_query::build_info();
    Ok(DatabaseOverview {
        path: s.url.clone(),
        connection_mode: "server".into(),
        opened_at: s.opened_at.to_rfc3339(),
        engine_version: engine.engine_version.to_string(),
        query_version: query.query_version.to_string(),
        git_sha: engine.mongreldb_git_sha.to_string(),
        table_count: tables.len(),
        tables,
        embedding_providers: Vec::new(),
        credentials_required: false,
        display_label: s.url.clone(),
    })
}

fn server_table_summary(s: &ServerSession, name: &str) -> AppResult<TableSummary> {
    let schema = s
        .client
        .kit_schema(name)
        .map_err(|e| AppError::msg(format!("schema {name}: {e}")))?;
    let row_count = s
        .client
        .count(name)
        .map_err(|e| AppError::msg(format!("count {name}: {e}")))?;
    let mut has_ann = false;
    let mut has_sparse = false;
    let mut has_minhash = false;
    let mut has_fm = false;
    let mut has_bitmap = false;
    let mut has_learned_range = false;
    for idx in &schema.indexes {
        let k = idx.kind.to_ascii_lowercase();
        if k.contains("ann") {
            has_ann = true;
        } else if k.contains("sparse") {
            has_sparse = true;
        } else if k.contains("minhash") {
            has_minhash = true;
        } else if k.contains("fm") {
            has_fm = true;
        } else if k.contains("bitmap") {
            has_bitmap = true;
        } else if k.contains("learned") || k.contains("range") || k.contains("pgm") {
            has_learned_range = true;
        }
    }
    let embedding_dims: Vec<u32> = schema
        .columns
        .iter()
        .filter_map(|c| parse_embedding_dim(&c.ty))
        .collect();
    Ok(TableSummary {
        name: name.to_string(),
        row_count,
        column_count: schema.columns.len(),
        index_count: schema.indexes.len(),
        has_ann,
        has_sparse,
        has_minhash,
        has_fm,
        has_bitmap,
        has_learned_range,
        embedding_dims,
    })
}

fn server_table_detail(s: &ServerSession, name: &str) -> AppResult<TableDetail> {
    let schema = s
        .client
        .kit_schema(name)
        .map_err(|e| AppError::msg(format!("schema {name}: {e}")))?;
    let row_count = s
        .client
        .count(name)
        .map_err(|e| AppError::msg(format!("count {name}: {e}")))?;
    let columns: Vec<ColumnInfo> = schema
        .columns
        .iter()
        .map(|c| {
            let mut flags = Vec::new();
            if c.primary_key {
                flags.push("PRIMARY_KEY".into());
            }
            if c.nullable {
                flags.push("NULLABLE".into());
            }
            if c.auto_increment {
                flags.push("AUTO_INCREMENT".into());
            }
            ColumnInfo {
                id: c.id,
                name: c.name.clone(),
                type_name: c.ty.clone(),
                flags,
                embedding_dim: parse_embedding_dim(&c.ty),
                embedding_source: c
                    .embedding_source
                    .as_ref()
                    .map(crate::db::inspect::describe_embedding_source),
            }
        })
        .collect();
    let col_name = |id: u16| {
        schema
            .columns
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("col_{id}"))
    };
    let indexes: Vec<IndexInfo> = schema
        .indexes
        .iter()
        .map(|idx| {
            crate::db::inspect::index_info_from_parts(
                idx.name.clone(),
                idx.column_id,
                col_name(idx.column_id),
                idx.kind.clone(),
                idx.predicate.clone(),
                &idx.options,
            )
        })
        .collect();
    let mut radar = IndexRadar {
        bitmap: 0,
        learned_range: 0,
        fm_index: 0,
        ann: 0,
        sparse: 0,
        minhash: 0,
    };
    for idx in &schema.indexes {
        let k = idx.kind.to_ascii_lowercase();
        if k.contains("ann") {
            radar.ann += 1;
        } else if k.contains("sparse") {
            radar.sparse += 1;
        } else if k.contains("minhash") {
            radar.minhash += 1;
        } else if k.contains("fm") {
            radar.fm_index += 1;
        } else if k.contains("bitmap") {
            radar.bitmap += 1;
        } else if k.contains("learned") || k.contains("range") || k.contains("pgm") {
            radar.learned_range += 1;
        }
    }
    Ok(TableDetail {
        name: name.to_string(),
        schema_id: schema.schema_id,
        row_count,
        columns,
        indexes,
        index_radar: radar,
        foreign_keys: Vec::new(),
    })
}

fn server_constellation(s: &ServerSession) -> AppResult<ConstellationGraph> {
    let names = s
        .client
        .list_tables()
        .map_err(|e| AppError::msg(format!("list_tables: {e}")))?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let host = s
        .url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .to_string();
    nodes.push(GraphNode {
        id: "db".into(),
        label: host,
        kind: "database".into(),
        meta: serde_json::json!({ "path": s.url, "tables": names.len(), "mode": "server" }),
    });
    for name in names {
        let detail = server_table_detail(s, &name)?;
        let table_id = format!("table:{name}");
        nodes.push(GraphNode {
            id: table_id.clone(),
            label: name.clone(),
            kind: "table".into(),
            meta: serde_json::json!({
                "rows": detail.row_count,
                "columns": detail.columns.len(),
                "indexes": detail.indexes.len(),
                "hasAnn": detail.index_radar.ann > 0,
            }),
        });
        edges.push(GraphEdge {
            from: "db".into(),
            to: table_id.clone(),
            kind: "owns".into(),
        });
        for col in &detail.columns {
            let col_id = format!("col:{name}:{}", col.name);
            nodes.push(GraphNode {
                id: col_id.clone(),
                label: col.name.clone(),
                kind: if col.embedding_dim.is_some() {
                    "embedding".into()
                } else {
                    "column".into()
                },
                meta: serde_json::json!({ "type": col.type_name, "flags": col.flags, "dim": col.embedding_dim }),
            });
            edges.push(GraphEdge {
                from: table_id.clone(),
                to: col_id,
                kind: "column".into(),
            });
        }
        for idx in &detail.indexes {
            let idx_id = format!("idx:{name}:{}", idx.name);
            nodes.push(GraphNode {
                id: idx_id.clone(),
                label: format!("{} ({})", idx.name, idx.kind),
                kind: "index".into(),
                meta: serde_json::json!({ "kind": idx.kind, "column": idx.column_name }),
            });
            edges.push(GraphEdge {
                from: table_id.clone(),
                to: idx_id.clone(),
                kind: "index".into(),
            });
            edges.push(GraphEdge {
                from: idx_id,
                to: format!("col:{name}:{}", idx.column_name),
                kind: "covers".into(),
            });
        }
        for fk in &detail.foreign_keys {
            edges.push(GraphEdge {
                from: table_id.clone(),
                to: format!("table:{}", fk.ref_table),
                kind: "fk".into(),
            });
        }
    }
    Ok(ConstellationGraph { nodes, edges })
}

fn server_run_sql(client: &MongrelClient, sql: &str, max_rows: usize) -> AppResult<SqlResult> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err(AppError::sql("empty SQL"));
    }
    let started = Instant::now();
    let kind = classify(sql);
    let batches = client
        .sql(sql)
        .map_err(|e| AppError::sql(format!("server SQL: {e}")))?;
    let (columns, mut rows) = crate::db::sql::batches_to_rows_public(&batches)?;
    let total = rows.len();
    let truncated = total > max_rows;
    if truncated {
        rows.truncate(max_rows);
    }
    Ok(SqlResult {
        columns,
        rows,
        row_count: total.min(max_rows),
        truncated,
        elapsed_ms: started.elapsed().as_millis() as u64,
        statement_kind: kind,
    })
}

fn classify(sql: &str) -> String {
    let head = sql
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    match head.as_str() {
        "SELECT" | "WITH" | "EXPLAIN" | "SHOW" | "DESCRIBE" | "DESC" | "VALUES" => "query".into(),
        "INSERT" | "UPDATE" | "DELETE" | "MERGE" | "TRUNCATE" => "dml".into(),
        "CREATE" | "ALTER" | "DROP" | "RENAME" => "ddl".into(),
        other if other.is_empty() => "empty".into(),
        other => other.to_ascii_lowercase(),
    }
}

fn parse_embedding_dim(ty: &str) -> Option<u32> {
    let lower = ty.to_ascii_lowercase();
    if !lower.contains("embedding") {
        return None;
    }
    // "Embedding(384)" or "embedding { dim: 384 }" etc.
    let digits: String = ty.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}
