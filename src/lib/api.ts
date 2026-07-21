import { invoke } from "@tauri-apps/api/core";

export type DatabaseOverview = {
  path: string;
  /** "direct" | "server" */
  connectionMode: string;
  openedAt: string;
  engineVersion: string;
  queryVersion: string;
  gitSha: string;
  tableCount: number;
  tables: TableSummary[];
  embeddingProviders: ProviderInfo[];
  credentialsRequired: boolean;
  displayLabel: string;
};

export type TableSummary = {
  name: string;
  rowCount: number;
  columnCount: number;
  indexCount: number;
  hasAnn: boolean;
  hasSparse: boolean;
  hasMinhash: boolean;
  hasFm: boolean;
  hasBitmap: boolean;
  hasLearnedRange: boolean;
  embeddingDims: number[];
};

export type TableDetail = {
  name: string;
  schemaId: number;
  rowCount: number;
  columns: ColumnInfo[];
  indexes: IndexInfo[];
  indexRadar: IndexRadar;
  foreignKeys?: ForeignKeyInfo[];
};

export type ForeignKeyInfo = {
  name: string;
  columns: string[];
  refTable: string;
  refColumns: string[];
};

export type ColumnInfo = {
  id: number;
  name: string;
  typeName: string;
  flags: string[];
  embeddingDim?: number | null;
  /** How vectors are produced (e.g. supplied_by_application, configured_model · …). */
  embeddingSource?: string | null;
};

/** ANN options from schema (algorithm × quantization since MongrelDB 0.63). */
export type AnnIndexOptions = {
  m: number;
  efConstruction: number;
  efSearch: number;
  /** "dense", "binary_sign", or "product". */
  quantization: string;
  /** "hnsw" | "diskann" | "ivf" */
  algorithm?: string;
  productNumSubvectors?: number | null;
  productBits?: number | null;
  diskannR?: number | null;
  ivfNlist?: number | null;
  ivfNprobe?: number | null;
};

export type IndexInfo = {
  name: string;
  columnId: number;
  columnName: string;
  kind: string;
  predicate?: string | null;
  /** Present for ANN indexes. */
  ann?: AnnIndexOptions | null;
  /** Short human summary of kind-specific options. */
  optionsSummary?: string | null;
};

export type IndexRadar = {
  bitmap: number;
  learnedRange: number;
  fmIndex: number;
  ann: number;
  sparse: number;
  minhash: number;
};

export type SqlResult = {
  columns: string[];
  rows: unknown[][];
  rowCount: number;
  truncated: boolean;
  elapsedMs: number;
  statementKind: string;
};

export type ProviderInfo = {
  providerId: string;
  modelId: string;
  modelVersion: string;
  dimension: number;
  health: string;
  backend: string;
};

export type ConstellationGraph = {
  nodes: { id: string; label: string; kind: string; meta: Record<string, unknown> }[];
  edges: { from: string; to: string; kind: string }[];
};

export type ChatMessage = {
  role: string;
  content: string;
  toolCallId?: string | null;
  name?: string | null;
  toolCalls?: unknown[] | null;
};

export type ChatConfig = {
  baseUrl: string;
  apiKey: string;
  model: string;
  systemPrompt?: string | null;
};

export type ChatResponse = {
  messages: ChatMessage[];
  toolTraces: { name: string; arguments: unknown; result: unknown; ok: boolean }[];
  model: string;
};

export type McpStatus = {
  running: boolean;
  mode: string;
  endpoint?: string | null;
  tools: string[];
  connections: number;
};

export type InstallAnnResult = {
  table: string;
  embeddingColumn: string;
  dimension: number;
  indexName: string;
  rowsEmbedded: number;
  message: string;
  /** True when the table already had a durable ANN index. */
  alreadyReady?: boolean;
  /** "dense", "binary_sign", or "product". */
  quantization?: string;
  /** "hnsw" | "diskann" | "ivf" */
  algorithm?: string;
  /** True when an existing ANN was dropped and recreated. */
  rebuilt?: boolean;
};

export type ReindexResult = {
  target: string;
  message: string;
  elapsedMs: number;
};

export async function appInfo() {
  return invoke<Record<string, unknown>>("app_info");
}

export async function getDemoUsed() {
  return invoke<boolean>("get_demo_used");
}

export async function setDemoUsed(used: boolean) {
  return invoke<void>("set_demo_used", { used });
}

export async function openDatabase(req: {
  path: string;
  username?: string;
  password?: string;
  passphrase?: string;
  createIfMissing?: boolean;
}) {
  return invoke<DatabaseOverview>("open_database", {
    req: {
      ...req,
      createIfMissing: req.createIfMissing ?? false,
    },
  });
}

export async function openServer(req: {
  url: string;
  bearerToken?: string;
  username?: string;
  password?: string;
}) {
  return invoke<DatabaseOverview>("open_server", { req });
}

export async function closeDatabase() {
  return invoke<void>("close_database");
}

export async function createDemo(path: string, withAnn: boolean) {
  return invoke<DatabaseOverview>("create_demo", { req: { path, withAnn } });
}

export async function getOverview() {
  return invoke<DatabaseOverview>("get_overview");
}

export async function getTable(name: string) {
  return invoke<TableDetail>("get_table", { name });
}

export async function getConstellation() {
  return invoke<ConstellationGraph>("get_constellation");
}

export type DbInsights = {
  cards: InsightCard[];
  suggestedQueries: SuggestedQuery[];
  connectionMode: string;
  label: string;
};

export type InsightCard = {
  title: string;
  value: string;
  detail: string;
  accent: string;
  sql?: string | null;
};

export type SuggestedQuery = {
  title: string;
  description: string;
  sql: string;
  category: string;
};

export async function getInsights() {
  return invoke<DbInsights>("get_insights");
}

export async function executeSql(sql: string, maxRows = 500) {
  return invoke<SqlResult>("execute_sql", { req: { sql, maxRows } });
}

export async function installDenseAnn(req: {
  table: string;
  embeddingColumn?: string;
  dimension?: number;
  sourceTextColumn?: string;
  providerId?: string;
  backfillLimit?: number;
  /** "dense" (default), "binary_sign", or "product". */
  quantization?: "dense" | "binary_sign" | "product";
  /** "hnsw" (default), "diskann", or "ivf". */
  algorithm?: "hnsw" | "diskann" | "ivf";
  productNumSubvectors?: number;
  productBits?: number;
  diskannR?: number;
  diskannL?: number;
  diskannBeamWidth?: number;
  ivfNlist?: number;
  ivfNprobe?: number;
  m?: number;
  efConstruction?: number;
  efSearch?: number;
  /** Drop existing ANN and recreate with the requested options. */
  rebuild?: boolean;
}) {
  return invoke<InstallAnnResult>("install_dense_ann", { req });
}

/** Engine REINDEX (analyze + compact + GC). Omit table for whole database. */
export async function reindexDatabase(table?: string) {
  return invoke<ReindexResult>("reindex_database", {
    req: { table: table || null },
  });
}

export async function semanticSearch(req: {
  table: string;
  embeddingColumn: string;
  query: string;
  k?: number;
  exactRerank?: boolean;
  /** Cosine similarity floor (exact path). Weak hits below this are dropped. */
  minScore?: number;
  projection?: string;
}) {
  return invoke<SqlResult>("semantic_search", { req });
}

export async function ensureLocalEmbeddings() {
  return invoke<ProviderInfo[]>("ensure_local_embeddings");
}

export async function listEmbeddingModels() {
  return invoke<{ local: { id: string; label: string; dimension: number; default: boolean }[]; note: string }>(
    "list_embedding_models",
  );
}

export async function chatCompletion(messages: ChatMessage[], config: ChatConfig) {
  return invoke<ChatResponse>("chat_completion", { req: { messages, config } });
}

export async function probeChat(config: ChatConfig) {
  return invoke<Record<string, unknown>>("probe_chat", { cfg: config });
}

export async function startMcp(mode = "http", host = "127.0.0.1", port = 7337) {
  return invoke<McpStatus>("start_mcp", { req: { mode, host, port } });
}

export async function stopMcp() {
  return invoke<McpStatus>("stop_mcp");
}

export async function mcpStatus() {
  return invoke<McpStatus>("mcp_status");
}

export async function mcpConfigSnippet() {
  return invoke<Record<string, unknown>>("mcp_config_snippet");
}

export type AboutInfo = {
  appName: string;
  version: string;
  license: string;
  repository: string;
  gitSha: string;
  engineVersion: string;
  queryVersion: string;
  description: string;
  tagline: string;
  platform: string;
};

export type LicenseDocMeta = {
  id: string;
  title: string;
  subtitle: string;
};

export type CrateCredit = {
  name: string;
  version: string;
  license: string;
  repository: string;
};

export type NpmPackageCredit = {
  name: string;
  version: string;
  license: string;
  repository: string;
  /** `"runtime"` production deps, `"dev"` build tooling. */
  role: string;
};

export type RuntimeComponent = {
  name: string;
  licenses: string;
  spdx: string[];
  projectUrl: string;
  notes: string;
};

export type CreditsData = {
  crates: CrateCredit[];
  packages: NpmPackageCredit[];
  runtime: RuntimeComponent[];
  crateCount: number;
  packageCount: number;
  runtimeCount: number;
};

export async function aboutInfo() {
  return invoke<AboutInfo>("about_info");
}

export async function licenseDocs() {
  return invoke<LicenseDocMeta[]>("license_docs");
}

export async function licenseDocument(id: string) {
  return invoke<string>("license_document", { id });
}

export async function creditsData() {
  return invoke<CreditsData>("credits_data");
}

export async function runtimeLicenseText(spdxId: string) {
  return invoke<string>("runtime_license_text", { spdxId });
}
