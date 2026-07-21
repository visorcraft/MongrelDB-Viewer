use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRequest {
    pub path: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub passphrase: Option<String>,
    /// When true, create a new multi-table database root if path is empty.
    #[serde(default)]
    pub create_if_missing: bool,
}

/// Connect to a running `mongreldb-server` (multi-client HTTP).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerOpenRequest {
    /// e.g. `http://127.0.0.1:8453`
    pub url: String,
    pub bearer_token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseOverview {
    /// Directory path (direct) or server URL (server).
    pub path: String,
    /// `"direct"` exclusive embedded open, or `"server"` multi-client HTTP.
    pub connection_mode: String,
    pub opened_at: String,
    pub engine_version: String,
    pub query_version: String,
    pub git_sha: String,
    pub table_count: usize,
    pub tables: Vec<TableSummary>,
    pub embedding_providers: Vec<ProviderInfo>,
    pub credentials_required: bool,
    /// Human label for the top bar.
    pub display_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableSummary {
    pub name: String,
    pub row_count: u64,
    pub column_count: usize,
    pub index_count: usize,
    pub has_ann: bool,
    pub has_sparse: bool,
    pub has_minhash: bool,
    pub has_fm: bool,
    pub has_bitmap: bool,
    pub has_learned_range: bool,
    pub embedding_dims: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    pub id: u16,
    pub name: String,
    pub type_name: String,
    pub flags: Vec<String>,
    pub embedding_dim: Option<u32>,
    /// Human description of how vectors are produced for this column
    /// (e.g. `supplied_by_application`, `configured_model · …`).
    pub embedding_source: Option<String>,
}

/// ANN HNSW options from schema (`IndexDef.options.ann`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnIndexOptions {
    pub m: usize,
    pub ef_construction: usize,
    pub ef_search: usize,
    /// `"dense"` (full f32 cosine) or `"binary_sign"` (legacy Hamming).
    pub quantization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub name: String,
    pub column_id: u16,
    pub column_name: String,
    pub kind: String,
    pub predicate: Option<String>,
    /// Present when `kind` is ANN and options were stored on the index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ann: Option<AnnIndexOptions>,
    /// Short human summary of kind-specific options (ANN / MinHash / LearnedRange).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableDetail {
    pub name: String,
    pub schema_id: u64,
    pub row_count: u64,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub index_radar: IndexRadar,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKeyInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexRadar {
    pub bitmap: usize,
    pub learned_range: usize,
    pub fm_index: usize,
    pub ann: usize,
    pub sparse: usize,
    pub minhash: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlRequest {
    pub sql: String,
    pub max_rows: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub truncated: bool,
    pub elapsed_ms: u64,
    pub statement_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub provider_id: String,
    pub model_id: String,
    pub model_version: String,
    pub dimension: u32,
    pub health: String,
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedRequest {
    pub texts: Vec<String>,
    pub provider_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedResponse {
    pub vectors: Vec<Vec<f32>>,
    pub dimension: u32,
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallAnnRequest {
    pub table: String,
    /// Column to store dense vectors. Default: "embedding".
    pub embedding_column: Option<String>,
    /// Dimension (default 384 for all-MiniLM-L6-v2).
    pub dimension: Option<u32>,
    /// Optional text column to backfill embeddings from after install.
    pub source_text_column: Option<String>,
    pub provider_id: Option<String>,
    pub index_name: Option<String>,
    pub m: Option<usize>,
    pub ef_construction: Option<usize>,
    pub ef_search: Option<usize>,
    pub backfill_limit: Option<usize>,
    /// `"dense"` (default, full f32 cosine) or `"binary_sign"` (legacy compact).
    pub quantization: Option<String>,
    /// Drop existing ANN on the embedding column and recreate with the requested
    /// options (quantization / m / ef_*). Use to upgrade BinarySign → Dense.
    #[serde(default)]
    pub rebuild: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallAnnResult {
    pub table: String,
    pub embedding_column: String,
    pub dimension: u32,
    pub index_name: String,
    pub rows_embedded: u64,
    pub message: String,
    /// True when the table already had an ANN index and no rebuild/DDL was needed.
    #[serde(default)]
    pub already_ready: bool,
    /// Quantization used for the index (`dense` or `binary_sign`).
    #[serde(default)]
    pub quantization: String,
    /// True when an existing ANN index was dropped and recreated.
    #[serde(default)]
    pub rebuilt: bool,
}

/// Run engine `REINDEX` maintenance (analyze + compact + GC).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexRequest {
    /// Table name. Omit or empty for whole-database `REINDEX`.
    pub table: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexResult {
    /// `"database"` or the table name.
    pub target: String,
    pub message: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSearchRequest {
    pub table: String,
    pub embedding_column: String,
    pub query: String,
    /// Max neighbors to return (top-k). Always ≤k rows, never "all matches".
    pub k: Option<usize>,
    pub provider_id: Option<String>,
    pub projection: Option<String>,
    /// When true (default), use `ann_search_exact` so hits are cosine-ranked.
    pub exact_rerank: Option<bool>,
    /// Optional cosine-similarity floor (exact path only). Drops weak hits so
    /// unrelated queries like "banana" do not fill the entire top-k list.
    pub min_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub config: ChatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    pub messages: Vec<ChatMessage>,
    pub tool_traces: Vec<ToolTrace>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolTrace {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: serde_json::Value,
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    pub mode: String,
    pub endpoint: Option<String>,
    pub tools: Vec<String>,
    pub connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStartRequest {
    /// "stdio" or "http"
    pub mode: String,
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDemoRequest {
    pub path: String,
    pub with_ann: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstellationGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub meta: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModels {
    pub local: Vec<LocalModelInfo>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelInfo {
    pub id: String,
    pub label: String,
    pub dimension: u32,
    pub default: bool,
}

/// At-a-glance analytics for the Overview deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbInsights {
    pub cards: Vec<InsightCard>,
    pub suggested_queries: Vec<SuggestedQuery>,
    pub connection_mode: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightCard {
    pub title: String,
    pub value: String,
    pub detail: String,
    pub accent: String,
    /// Optional SQL to run when the card is clicked.
    pub sql: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedQuery {
    pub title: String,
    pub description: String,
    pub sql: String,
    pub category: String,
}
