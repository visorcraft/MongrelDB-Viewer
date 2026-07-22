use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tauri::State;

use crate::chat;
use crate::db::{self, Connection, OpenMode};
use crate::embeddings::{EmbeddingHub, RemoteEmbedConfig};
use crate::error::{AppError, AppResult};
use crate::mcp::server::{self as mcp_server, McpHandle};
use crate::mcp::tools::ToolExecutor;
use crate::models::*;

pub struct AppState {
    pub db: Arc<RwLock<Option<Connection>>>,
    pub embeddings: EmbeddingHub,
    pub mcp: Mutex<Option<McpHandle>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: Arc::new(RwLock::new(None)),
            embeddings: EmbeddingHub::default(),
            mcp: Mutex::new(None),
        }
    }

    pub fn executor(&self) -> ToolExecutor {
        ToolExecutor::new(Arc::clone(&self.db), self.embeddings.clone())
    }
}

fn viewer_flags_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mongreldb-viewer")
        .join("flags.json")
}

fn read_flags() -> serde_json::Value {
    let path = viewer_flags_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn write_flags(value: &serde_json::Value) -> AppResult<()> {
    let path = viewer_flags_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(value)?;
    std::fs::write(path, body)?;
    Ok(())
}

#[tauri::command]
pub fn get_demo_used() -> bool {
    read_flags()
        .get("demoUsed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[tauri::command]
pub fn set_demo_used(used: bool) -> AppResult<()> {
    let mut flags = read_flags();
    if let Some(obj) = flags.as_object_mut() {
        obj.insert("demoUsed".into(), serde_json::Value::Bool(used));
    } else {
        flags = serde_json::json!({ "demoUsed": used });
    }
    write_flags(&flags)
}

#[tauri::command]
pub fn app_info() -> serde_json::Value {
    let engine = mongreldb_core::build_info();
    let query = mongreldb_query::build_info();
    serde_json::json!({
        "app": "MongrelDB Viewer",
        "version": env!("CARGO_PKG_VERSION"),
        "engineVersion": engine.engine_version,
        "queryVersion": query.query_version,
        "gitSha": engine.mongreldb_git_sha,
        "client": "rust direct (mongreldb-core) + server (mongreldb-client)",
        "defaultEmbedding": {
            "model": crate::embeddings::DEFAULT_MODEL_ID,
            "dimension": crate::embeddings::DEFAULT_DIM,
            "providerId": crate::embeddings::DEFAULT_PROVIDER_ID,
        }
    })
}

#[tauri::command]
pub fn open_database(state: State<'_, AppState>, req: OpenRequest) -> AppResult<DatabaseOverview> {
    let mode = if req.create_if_missing {
        OpenMode::Create
    } else {
        OpenMode::Open
    };
    {
        let mut g = state.db.write();
        *g = None;
    }
    let conn = Connection::open_direct(
        &req.path,
        req.username.as_deref(),
        req.password.as_deref(),
        req.passphrase.as_deref(),
        mode,
    )?;
    let mut overview = conn.overview()?;
    overview.embedding_providers = state.embeddings.list_providers();
    *state.db.write() = Some(conn);
    Ok(overview)
}

#[tauri::command]
pub fn open_server(
    state: State<'_, AppState>,
    req: ServerOpenRequest,
) -> AppResult<DatabaseOverview> {
    {
        let mut g = state.db.write();
        *g = None;
    }
    let conn = Connection::open_server(&req)?;
    let mut overview = conn.overview()?;
    overview.embedding_providers = state.embeddings.list_providers();
    *state.db.write() = Some(conn);
    Ok(overview)
}

#[tauri::command]
pub fn close_database(state: State<'_, AppState>) -> AppResult<()> {
    *state.db.write() = None;
    Ok(())
}

#[tauri::command]
pub fn create_demo(
    state: State<'_, AppState>,
    req: CreateDemoRequest,
) -> AppResult<DatabaseOverview> {
    {
        let mut g = state.db.write();
        *g = None;
    }
    let conn = Connection::create_demo(&req.path, req.with_ann)?;
    let mut overview = conn.overview()?;
    overview.embedding_providers = state.embeddings.list_providers();
    *state.db.write() = Some(conn);
    // Persist so "Create demo DB" stays hidden after the first successful run.
    let _ = set_demo_used(true);
    Ok(overview)
}

#[tauri::command]
pub fn get_overview(state: State<'_, AppState>) -> AppResult<DatabaseOverview> {
    let guard = state.db.read();
    let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
    let mut overview = conn.overview()?;
    overview.embedding_providers = state.embeddings.list_providers();
    Ok(overview)
}

#[tauri::command]
pub fn get_table(state: State<'_, AppState>, name: String) -> AppResult<TableDetail> {
    let guard = state.db.read();
    let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
    conn.table_detail(&name)
}

#[tauri::command]
pub fn get_constellation(state: State<'_, AppState>) -> AppResult<ConstellationGraph> {
    let guard = state.db.read();
    let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
    conn.constellation()
}

#[tauri::command]
pub async fn get_insights(state: State<'_, AppState>) -> AppResult<DbInsights> {
    // Snapshot the connection (clone Arcs / HTTP client) so we never hold the
    // parking_lot guard across an await.
    let conn = {
        let guard = state.db.read();
        let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
        match conn {
            Connection::Direct(d) => Connection::Direct(db::DbSession {
                path: d.path.clone(),
                database: std::sync::Arc::clone(&d.database),
                session: std::sync::Arc::clone(&d.session),
                opened_at: d.opened_at,
                credentials_required: d.credentials_required,
            }),
            Connection::Server(s) => Connection::Server(crate::db::connection::ServerSession {
                url: s.url.clone(),
                client: s.client.clone(),
                opened_at: s.opened_at,
                health: s.health.clone(),
            }),
        }
    };
    db::build_insights(&conn).await
}

#[tauri::command]
pub async fn execute_sql(state: State<'_, AppState>, req: SqlRequest) -> AppResult<SqlResult> {
    let work = {
        let guard = state.db.read();
        let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
        conn.sql_work(req)?
    };
    work.run().await
}

#[tauri::command]
pub async fn install_dense_ann(
    state: State<'_, AppState>,
    req: InstallAnnRequest,
) -> AppResult<InstallAnnResult> {
    let direct = {
        let guard = state.db.read();
        let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
        let d = conn.as_direct()?;
        // rebuild a lightweight view
        db::DbSession {
            path: d.path.clone(),
            database: std::sync::Arc::clone(&d.database),
            session: std::sync::Arc::clone(&d.session),
            opened_at: d.opened_at,
            credentials_required: d.credentials_required,
        }
    };
    db::install_dense_ann(&direct, &state.embeddings, req).await
}

/// Engine `REINDEX` / `REINDEX <table>` (analyze + compact + GC).
#[tauri::command]
pub async fn reindex_database(
    state: State<'_, AppState>,
    req: ReindexRequest,
) -> AppResult<ReindexResult> {
    // Prefer direct session; fall back to SQL over the open connection (server).
    let direct = {
        let guard = state.db.read();
        let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
        match conn {
            Connection::Direct(d) => Some(db::DbSession {
                path: d.path.clone(),
                database: std::sync::Arc::clone(&d.database),
                session: std::sync::Arc::clone(&d.session),
                opened_at: d.opened_at,
                credentials_required: d.credentials_required,
            }),
            Connection::Server(_) => None,
        }
    };
    if let Some(direct) = direct {
        return db::reindex(&direct, req).await;
    }
    let table = req
        .table
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let sql = match table {
        None => "REINDEX".to_string(),
        Some(name) => {
            if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(AppError::msg(format!(
                    "invalid table name {name:?}: use letters, digits, and underscore only"
                )));
            }
            format!("REINDEX {name}")
        }
    };
    let target = table.unwrap_or("database").to_string();
    let work = {
        let guard = state.db.read();
        let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
        conn.sql_work(SqlRequest {
            sql,
            max_rows: Some(1),
        })?
    };
    let started = std::time::Instant::now();
    work.run().await?;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(ReindexResult {
        message: if target == "database" {
            format!("REINDEX completed for the entire database in {elapsed_ms} ms.")
        } else {
            format!("REINDEX completed for table `{target}` in {elapsed_ms} ms.")
        },
        target,
        elapsed_ms,
    })
}

#[tauri::command]
pub async fn semantic_search(
    state: State<'_, AppState>,
    req: SemanticSearchRequest,
) -> AppResult<SqlResult> {
    // Shared path with MCP: honors exact_rerank → ann_search_exact (ranked).
    db::semantic_search_on_connection(&state.db, &state.embeddings, req).await
}

#[tauri::command]
pub fn ensure_local_embeddings(state: State<'_, AppState>) -> AppResult<Vec<ProviderInfo>> {
    state.embeddings.ensure_local_default()?;
    Ok(state.embeddings.list_providers())
}

#[tauri::command]
pub fn configure_remote_embeddings(
    state: State<'_, AppState>,
    cfg: RemoteEmbedConfig,
) -> AppResult<Vec<ProviderInfo>> {
    state.embeddings.configure_remote(cfg);
    Ok(state.embeddings.list_providers())
}

#[tauri::command]
pub fn list_embedding_models() -> AvailableModels {
    AvailableModels {
        local: EmbeddingHub::available_models(),
        note: "Default dense ANN uses all-MiniLM-L6-v2 (384 dimensions). Install the local model on demand, or point at any OpenAI-compatible embeddings endpoint.".into(),
    }
}

#[tauri::command]
pub fn embed_texts(state: State<'_, AppState>, req: EmbedRequest) -> AppResult<EmbedResponse> {
    state
        .embeddings
        .embed(&req.texts, req.provider_id.as_deref())
}

#[tauri::command]
pub async fn chat_completion(
    state: State<'_, AppState>,
    req: ChatRequest,
) -> AppResult<ChatResponse> {
    let executor = state.executor();
    chat::chat(&executor, req).await
}

#[tauri::command]
pub async fn probe_chat(cfg: ChatConfig) -> AppResult<serde_json::Value> {
    chat::probe(&cfg).await
}

#[tauri::command]
pub async fn start_mcp(state: State<'_, AppState>, req: McpStartRequest) -> AppResult<McpStatus> {
    {
        let mut g = state.mcp.lock();
        if let Some(h) = g.take() {
            h.stop();
        }
    }

    let mode = req.mode.to_ascii_lowercase();
    if mode == "stdio" {
        return Err(AppError::Mcp(
            "stdio MCP is available by launching the binary with --mcp-stdio; use HTTP mode inside the app for a live endpoint".into(),
        ));
    }

    let host = req.host.unwrap_or_else(|| "127.0.0.1".into());
    let port = req.port.unwrap_or(7337);
    let executor = state.executor();
    let handle = mcp_server::start_http(executor, &host, port).await?;
    let status = handle.status();
    *state.mcp.lock() = Some(handle);
    Ok(status)
}

#[tauri::command]
pub fn stop_mcp(state: State<'_, AppState>) -> AppResult<McpStatus> {
    let mut g = state.mcp.lock();
    if let Some(h) = g.take() {
        h.stop();
    }
    Ok(McpStatus {
        running: false,
        mode: "stopped".into(),
        endpoint: None,
        tools: crate::mcp::tools::tool_definitions()
            .iter()
            .filter_map(|t| {
                t.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect(),
        connections: 0,
    })
}

#[tauri::command]
pub fn mcp_status(state: State<'_, AppState>) -> McpStatus {
    let g = state.mcp.lock();
    if let Some(h) = g.as_ref() {
        h.status()
    } else {
        McpStatus {
            running: false,
            mode: "stopped".into(),
            endpoint: None,
            tools: crate::mcp::tools::tool_definitions()
                .iter()
                .filter_map(|t| {
                    t.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect(),
            connections: 0,
        }
    }
}

#[tauri::command]
pub fn mcp_config_snippet(state: State<'_, AppState>) -> serde_json::Value {
    let status = mcp_status(state);
    let endpoint = status
        .endpoint
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:7337/mcp".into());
    serde_json::json!({
        "claudeDesktop": {
            "mcpServers": {
                "mongreldb-viewer": {
                    "url": endpoint
                }
            }
        },
        "cursor": {
            "mcpServers": {
                "mongreldb-viewer": {
                    "url": endpoint
                }
            }
        },
        "stdioNote": "For stdio transport, run: mongreldb-viewer --mcp-stdio (database must already be open in a paired session or set MONGRELDB_VIEWER_PATH).",
        "endpoint": endpoint,
        "tools": status.tools,
    })
}

#[tauri::command]
pub fn about_info() -> crate::legal::AboutInfo {
    crate::legal::about_info()
}

#[tauri::command]
pub fn license_docs() -> Vec<crate::legal::LicenseDocMeta> {
    crate::legal::license_docs()
}

#[tauri::command]
pub fn license_document(id: String) -> AppResult<String> {
    crate::legal::license_document(&id)
}

#[tauri::command]
pub fn credits_data() -> AppResult<crate::legal::CreditsData> {
    crate::legal::credits_data()
}

#[tauri::command]
pub fn runtime_license_text(spdx_id: String) -> AppResult<String> {
    crate::legal::runtime_license_text(&spdx_id)
}
