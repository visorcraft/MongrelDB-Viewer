//! MCP server for MongrelDB Viewer.
//!
//! Transports:
//! - **HTTP + SSE** (default for the desktop app): local endpoint agents can attach to
//! - **stdio**: for `npx`/terminal MCP clients via a thin bridge command
//!
//! Implements the core MCP surface: `initialize`, `tools/list`, `tools/call`,
//! `ping`, and `notifications/initialized`.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::oneshot;

use crate::error::{AppError, AppResult};
use crate::mcp::tools::{tool_definitions, ToolExecutor};
use crate::models::McpStatus;

#[derive(Clone)]
pub struct McpServer {
    executor: Arc<ToolExecutor>,
    connections: Arc<AtomicUsize>,
}

impl McpServer {
    pub fn new(executor: ToolExecutor) -> Self {
        Self {
            executor: Arc::new(executor),
            connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn handle_rpc(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        self.connections.fetch_add(1, Ordering::Relaxed);
        let result = self.dispatch(req).await;
        self.connections.fetch_sub(1, Ordering::Relaxed);
        result
    }

    async fn dispatch(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();
        match req.method.as_str() {
            "initialize" => JsonRpcResponse::ok(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "mongreldb-viewer",
                        "version": env!("CARGO_PKG_VERSION"),
                        "title": "MongrelDB Viewer MCP"
                    },
                    "instructions": "Explore and query the MongrelDB database currently open in MongrelDB Viewer. Prefer list_tables / describe_table before SQL. Use semantic_search for ANN. Use install_dense_ann to add 384-d ANN (default algorithm=hnsw, quantization=dense; also diskann/ivf and product quantization)."
                }),
            ),
            "notifications/initialized" | "initialized" => {
                // Notification - no response body required, but HTTP always returns a value.
                JsonRpcResponse::ok(id, json!({}))
            }
            "ping" => JsonRpcResponse::ok(id, json!({})),
            "tools/list" => JsonRpcResponse::ok(
                id,
                json!({
                    "tools": tool_definitions()
                }),
            ),
            "tools/call" => {
                let name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let arguments = req
                    .params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let trace = self.executor.call(&name, arguments).await;
                let text = serde_json::to_string_pretty(&trace.result)
                    .unwrap_or_else(|_| "{}".into());
                JsonRpcResponse::ok(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": text
                        }],
                        "structuredContent": trace.result,
                        "isError": !trace.ok
                    }),
                )
            }
            "resources/list" => JsonRpcResponse::ok(id, json!({ "resources": [] })),
            "prompts/list" => JsonRpcResponse::ok(id, json!({ "prompts": [] })),
            other => JsonRpcResponse::err(id, -32601, format!("method not found: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }
    fn err(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

pub struct McpHandle {
    pub mode: String,
    pub endpoint: Option<String>,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    server: McpServer,
    join: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl McpHandle {
    pub fn status(&self) -> McpStatus {
        McpStatus {
            running: self.join.lock().as_ref().is_some_and(|j| !j.is_finished()),
            mode: self.mode.clone(),
            endpoint: self.endpoint.clone(),
            tools: tool_definitions()
                .iter()
                .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect(),
            connections: self.server.connections.load(Ordering::Relaxed),
        }
    }

    pub fn stop(&self) {
        if let Some(tx) = self.shutdown.lock().take() {
            let _ = tx.send(());
        }
    }
}

pub async fn start_http(
    executor: ToolExecutor,
    host: &str,
    port: u16,
) -> AppResult<McpHandle> {
    let server = McpServer::new(executor);
    let state = server.clone();

    let app = Router::new()
        .route("/", get(|| async { "MongrelDB Viewer MCP - POST /mcp for JSON-RPC" }))
        .route("/health", get(|| async { Json(json!({"ok": true, "service": "mongreldb-viewer-mcp"})) }))
        .route("/mcp", post(mcp_post))
        .route(
            "/sse",
            get(|| async {
                // Minimal SSE hello so clients can probe the endpoint.
                (
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "text/event-stream",
                    )],
                    "event: endpoint\ndata: /mcp\n\n",
                )
                    .into_response()
            }),
        )
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| AppError::Mcp(format!("invalid address: {e}")))?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Mcp(format!("bind {addr} failed: {e}")))?;
    let bound = listener
        .local_addr()
        .map_err(|e| AppError::Mcp(e.to_string()))?;
    let endpoint = format!("http://{bound}/mcp");

    let (tx, rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = rx.await;
        });
        if let Err(e) = server.await {
            tracing::error!("MCP HTTP server error: {e}");
        }
    });

    Ok(McpHandle {
        mode: "http".into(),
        endpoint: Some(endpoint),
        shutdown: Mutex::new(Some(tx)),
        server,
        join: Mutex::new(Some(join)),
    })
}

async fn mcp_post(
    State(server): State<McpServer>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(server.handle_rpc(req).await)
}

/// Run a single-shot stdio MCP session (blocking read loop). Used when the
/// binary is launched as `mongreldb-viewer --mcp-stdio`.
pub async fn run_stdio(executor: ToolExecutor) -> AppResult<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let server = McpServer::new(executor);
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| AppError::Mcp(e.to_string()))?
    {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                let err = JsonRpcResponse::err(Value::Null, -32700, format!("parse error: {e}"));
                let payload = serde_json::to_string(&err)?;
                stdout
                    .write_all(payload.as_bytes())
                    .await
                    .map_err(|e| AppError::Mcp(e.to_string()))?;
                stdout
                    .write_all(b"\n")
                    .await
                    .map_err(|e| AppError::Mcp(e.to_string()))?;
                stdout.flush().await.map_err(|e| AppError::Mcp(e.to_string()))?;
                continue;
            }
        };
        // Notifications may omit id; still process.
        let is_notification = req.id.is_null() && req.method.starts_with("notifications/");
        let resp = server.handle_rpc(req).await;
        if is_notification {
            continue;
        }
        let payload = serde_json::to_string(&resp)?;
        stdout
            .write_all(payload.as_bytes())
            .await
            .map_err(|e| AppError::Mcp(e.to_string()))?;
        stdout
            .write_all(b"\n")
            .await
            .map_err(|e| AppError::Mcp(e.to_string()))?;
        stdout.flush().await.map_err(|e| AppError::Mcp(e.to_string()))?;
    }
    Ok(())
}
