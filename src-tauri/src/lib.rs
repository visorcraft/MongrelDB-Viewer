mod chat;
mod commands;
mod db;
mod embeddings;
mod error;
mod legal;
mod linux_display;
mod mcp;
mod models;

use std::sync::Arc;

use commands::AppState;
use db::session::OpenMode;
use embeddings::EmbeddingHub;
use mcp::server::run_stdio;
use mcp::tools::ToolExecutor;
use parking_lot::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Must run before Tauri/WRY initializes GTK or WebKit.
    linux_display::apply_web_display_defaults();
    // Wayland taskbar: install FreeDesktop .desktop + hicolor helmet icons.
    linux_display::ensure_desktop_integration();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_demo_used,
            commands::set_demo_used,
            commands::open_database,
            commands::open_server,
            commands::close_database,
            commands::create_demo,
            commands::get_overview,
            commands::get_table,
            commands::get_constellation,
            commands::get_insights,
            commands::execute_sql,
            commands::install_dense_ann,
            commands::reindex_database,
            commands::semantic_search,
            commands::ensure_local_embeddings,
            commands::configure_remote_embeddings,
            commands::list_embedding_models,
            commands::embed_texts,
            commands::chat_completion,
            commands::probe_chat,
            commands::start_mcp,
            commands::stop_mcp,
            commands::mcp_status,
            commands::mcp_config_snippet,
            commands::about_info,
            commands::license_docs,
            commands::license_document,
            commands::credits_data,
            commands::runtime_license_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MongrelDB Viewer");
}

/// Headless MCP over stdio for terminal / IDE clients.
pub async fn run_mcp_stdio() -> Result<(), Box<dyn std::error::Error>> {
    use crate::db::Connection;

    let embeddings = EmbeddingHub::default();
    let db = Arc::new(RwLock::new(None));

    if let Ok(path) = std::env::var("MONGRELDB_VIEWER_PATH") {
        let conn = Connection::open_direct(&path, None, None, None, OpenMode::Open)?;
        *db.write() = Some(conn);
    } else if let Ok(url) = std::env::var("MONGRELDB_VIEWER_SERVER") {
        let conn = Connection::open_server(&crate::models::ServerOpenRequest {
            url,
            bearer_token: std::env::var("MONGRELDB_VIEWER_TOKEN").ok(),
            username: std::env::var("MONGRELDB_VIEWER_USER").ok(),
            password: std::env::var("MONGRELDB_VIEWER_PASSWORD").ok(),
        })?;
        *db.write() = Some(conn);
    }

    let executor = ToolExecutor::new(db, embeddings);
    run_stdio(executor).await?;
    Ok(())
}
