//! OpenAI-compatible chat with tool-calling against the open MongrelDB.

use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::mcp::tools::{openai_tools, ToolExecutor};
use crate::models::{ChatConfig, ChatMessage, ChatRequest, ChatResponse, ToolTrace};

const DEFAULT_SYSTEM: &str = r#"You are the MongrelDB Viewer co-pilot embedded in the Signal Deck.
You help users understand and interact with an open MongrelDB database.

MongrelDB strengths to surface when relevant:
- Log-structured columnar storage (.sr runs) with Bε-tree memtable + WAL group commit
- Six secondary index kinds sharing one RowId space: Bitmap, LearnedRange (PGM), FM-index, ANN (HNSW), Sparse, MinHash
- AI-native retrieval: ANN algorithms hnsw/diskann/ivf × quantizations dense/binary_sign/product; sparse; hybrid RRF; exact rerank
- DataFusion SQL with recursive CTEs, windows, scored AI table functions

Rules:
- Prefer tools over guessing schema. Call list_tables / describe_table first.
- Use execute_sql for precise queries. Prefer SELECT.
- Use semantic_search when meaning-based retrieval fits (requires an ANN index). On Direct opens this prefers engine-native retrieve_text (0.64 semantic identity + provenance) and falls back to SQL ann_search_exact.
- Use install_dense_ann only when the user wants 384-d ANN installed/backfilled (default algorithm=hnsw, quantization=dense; rebuild=true drops and recreates; product needs product_num_subvectors). Install stamps configured_model so retrieve_text can resolve the Viewer MiniLM provider.
- Use reindex for engine maintenance (REINDEX table or whole database: analyze + compact + GC).
- Be concise, technical, and accurate. Never invent vectors.
"#;

pub async fn chat(executor: &ToolExecutor, req: ChatRequest) -> AppResult<ChatResponse> {
    let cfg = req.config;
    if cfg.base_url.trim().is_empty() || cfg.model.trim().is_empty() {
        return Err(AppError::Chat(
            "base_url and model are required for OpenAI-compatible chat".into(),
        ));
    }

    let mut messages: Vec<Value> = Vec::new();
    let system = cfg
        .system_prompt
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SYSTEM.to_string());
    messages.push(json!({"role": "system", "content": system}));

    for m in &req.messages {
        let mut obj = json!({
            "role": m.role,
            "content": m.content,
        });
        if let Some(id) = &m.tool_call_id {
            obj["tool_call_id"] = json!(id);
        }
        if let Some(name) = &m.name {
            obj["name"] = json!(name);
        }
        if let Some(calls) = &m.tool_calls {
            obj["tool_calls"] = json!(calls);
        }
        messages.push(obj);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| AppError::Http(e.to_string()))?;

    let tools = openai_tools();
    let mut tool_traces: Vec<ToolTrace> = Vec::new();
    let mut out_messages: Vec<ChatMessage> = req.messages.clone();

    // Tool loop (bounded)
    for _round in 0..8 {
        let body = json!({
            "model": cfg.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "temperature": 0.2,
        });

        let url = chat_completions_url(&cfg.base_url);
        let mut request = client.post(&url).json(&body);
        if !cfg.api_key.is_empty() {
            request = request.bearer_auth(&cfg.api_key);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| AppError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| AppError::Http(e.to_string()))?;

        let payload: Value = resp
            .json()
            .await
            .map_err(|e| AppError::Http(e.to_string()))?;

        let choice = payload
            .pointer("/choices/0/message")
            .cloned()
            .ok_or_else(|| AppError::Chat(format!("unexpected chat response: {payload}")))?;

        let tool_calls = choice.get("tool_calls").and_then(|t| t.as_array()).cloned();
        let content = choice
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        // Append assistant message to both OpenAI transcript and UI transcript.
        messages.push(choice.clone());
        out_messages.push(ChatMessage {
            role: "assistant".into(),
            content: content.clone(),
            tool_call_id: None,
            name: None,
            tool_calls: tool_calls.clone(),
        });

        let Some(calls) = tool_calls else {
            // Final answer
            return Ok(ChatResponse {
                messages: out_messages,
                tool_traces,
                model: cfg.model,
            });
        };

        if calls.is_empty() {
            return Ok(ChatResponse {
                messages: out_messages,
                tool_traces,
                model: cfg.model,
            });
        }

        for call in calls {
            let id = call
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string();
            let name = call
                .pointer("/function/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let args_raw = call
                .pointer("/function/arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let args: Value = serde_json::from_str(args_raw).unwrap_or_else(|_| json!({}));
            let trace = executor.call(&name, args).await;
            let result_text = serde_json::to_string(&trace.result).unwrap_or_else(|_| "{}".into());

            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": result_text,
            }));
            out_messages.push(ChatMessage {
                role: "tool".into(),
                content: result_text,
                tool_call_id: Some(id),
                name: Some(name),
                tool_calls: None,
            });
            tool_traces.push(trace);
        }
    }

    Ok(ChatResponse {
        messages: out_messages,
        tool_traces,
        model: cfg.model,
    })
}

fn chat_completions_url(base: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

/// Validate config without sending a full chat (models list if available).
pub async fn probe(cfg: &ChatConfig) -> AppResult<Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Http(e.to_string()))?;
    let base = cfg.base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{base}/models")
    } else if base.contains("/v1/") {
        format!(
            "{}/models",
            base.rsplit_once("/v1").map(|(a, _)| a).unwrap_or(base)
        )
    } else {
        format!("{base}/v1/models")
    };
    let mut request = client.get(&url);
    if !cfg.api_key.is_empty() {
        request = request.bearer_auth(&cfg.api_key);
    }
    match request.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body: Value = resp.json().await.unwrap_or(json!({}));
            Ok(json!({ "ok": status < 400, "status": status, "body": body }))
        }
        Err(e) => Ok(json!({ "ok": false, "error": e.to_string() })),
    }
}
