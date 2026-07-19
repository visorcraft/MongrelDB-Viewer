// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Optional stdio MCP mode for terminal clients:
    //   mongreldb-viewer --mcp-stdio
    //   MONGRELDB_VIEWER_PATH=/path/to/db mongreldb-viewer --mcp-stdio
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp-stdio") {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            if let Err(e) = mongreldb_viewer_lib::run_mcp_stdio().await {
                eprintln!("MCP stdio error: {e}");
                std::process::exit(1);
            }
        });
        return;
    }

    // GUI path: linux_display defaults are applied inside run() before GTK.
    mongreldb_viewer_lib::run()
}
