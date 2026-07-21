import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  appInfo,
  chatCompletion,
  closeDatabase,
  createDemo,
  ensureLocalEmbeddings,
  executeSql,
  getConstellation,
  getDemoUsed,
  getInsights,
  getOverview,
  getTable,
  type ColumnInfo,
  installDenseAnn,
  listEmbeddingModels,
  mcpConfigSnippet,
  mcpStatus,
  openDatabase,
  openServer,
  probeChat,
  reindexDatabase,
  semanticSearch,
  setDemoUsed,
  startMcp,
  stopMcp,
  type ChatConfig,
  type ChatMessage,
  type ConstellationGraph,
  type DatabaseOverview,
  type DbInsights,
  type McpStatus,
  type SqlResult,
  type TableDetail,
} from "./lib/api";
import { loadRecents, pushRecent, type RecentConnection } from "./lib/recents";
import { paletteShortcutLabel, paletteShortcutParts } from "./lib/platform";
import { AboutHub } from "./components/AboutPages";
import { CommandPalette, type PaletteAction } from "./components/CommandPalette";
import { Constellation } from "./components/Constellation";
import { DataTable } from "./components/DataTable";
import { SqlWorkbench } from "./components/SqlWorkbench";
import { TableBrowser } from "./components/TableBrowser";

type View =
  | "deck"
  | "constellation"
  | "table"
  | "sql"
  | "ann"
  | "agent"
  | "mcp"
  | "about";

type AboutSub = "about" | "licenses" | "credits";

const NAV: { id: View; icon: string; label: string }[] = [
  { id: "deck", icon: "◈", label: "Deck" },
  { id: "constellation", icon: "✦", label: "Stars" },
  { id: "table", icon: "▤", label: "Table" },
  { id: "sql", icon: "Σ", label: "SQL" },
  { id: "ann", icon: "◎", label: "ANN" },
  { id: "agent", icon: "◉", label: "Agent" },
  { id: "mcp", icon: "⇄", label: "MCP" },
];

/** True for columns we can embed as text for dense ANN backfill. */
function isEmbeddableTextColumn(c: ColumnInfo): boolean {
  const t = c.typeName.toLowerCase();
  if (t.includes("embedding")) return false;
  if (t.includes("int") || t.includes("float") || t.includes("double") || t.includes("bool")) {
    return false;
  }
  return (
    t.includes("bytes") ||
    t.includes("string") ||
    t.includes("utf") ||
    t.includes("json") ||
    t.includes("text") ||
    t.includes("char")
  );
}

/** Prefer a real text-ish column on the selected ANN table (not a hard-coded `body`). */
function preferTextColumn(columns: ColumnInfo[], current: string): string {
  const textCols = textColumnOptions(columns);
  const names = textCols.map((c) => c.name);
  if (current && names.includes(current)) return current;
  const preferred = [
    "body",
    "text",
    "content",
    "title",
    "name",
    "payload",
    "kind",
    "message",
    "description",
    "summary",
  ];
  for (const p of preferred) {
    if (names.includes(p)) return p;
  }
  return names[0] ?? "";
}

function textColumnOptions(columns: ColumnInfo[]): ColumnInfo[] {
  return columns.filter(isEmbeddableTextColumn);
}

function chipClass(kind: string): string {
  const k = kind.toLowerCase();
  if (k.includes("bitmap")) return "bitmap";
  if (k.includes("learned") || k === "pgm") return "pgm";
  if (k.includes("fm")) return "fm";
  if (k === "ann" || k.includes("hnsw")) return "ann";
  if (k.includes("sparse")) return "sparse";
  if (k.includes("minhash") || k.includes("lsh")) return "minhash";
  return "bitmap";
}

function prettyIndexKind(kind: string): string {
  switch (kind) {
    case "Bitmap":
      return "Bitmap";
    case "LearnedRange":
      return "Range";
    case "FmIndex":
      return "Text";
    case "Ann":
      return "ANN";
    case "Sparse":
      return "Sparse";
    case "MinHash":
      return "MinHash";
    default:
      return kind;
  }
}

function prettyQuantization(q: string | undefined | null): string {
  if (!q) return "—";
  switch (q) {
    case "dense":
      return "Dense f32 cosine";
    case "binary_sign":
      return "BinarySign (Hamming)";
    default:
      return q;
  }
}

/** First ANN index on the table detail, if any. */
function annIndexOn(detail: TableDetail | null) {
  return detail?.indexes.find((i) => i.kind.toLowerCase().includes("ann")) ?? null;
}

export default function App() {
  const [view, setView] = useState<View>("deck");
  const [aboutSub, setAboutSub] = useState<AboutSub>("about");
  const [overview, setOverview] = useState<DatabaseOverview | null>(null);
  const [graph, setGraph] = useState<ConstellationGraph | null>(null);
  const [selectedTable, setSelectedTable] = useState<string>("");
  const [tableDetail, setTableDetail] = useState<TableDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [info, setInfo] = useState<Record<string, unknown> | null>(null);

  // SQL
  const [sql, setSql] = useState("SELECT name FROM information_schema.tables ORDER BY name");
  const [sqlResult, setSqlResult] = useState<SqlResult | null>(null);
  const [sqlHistory, setSqlHistory] = useState<string[]>([]);
  const [insights, setInsights] = useState<DbInsights | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [disconnectOpen, setDisconnectOpen] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [recents, setRecents] = useState<RecentConnection[]>(() => loadRecents());
  const [connectedAt, setConnectedAt] = useState<number | null>(null);
  const [demoUsed, setDemoUsedState] = useState(false);
  const contentRef = useRef<HTMLElement | null>(null);

  // ANN
  const [annTable, setAnnTable] = useState("");
  const [annTextCol, setAnnTextCol] = useState("body");
  const [annTableDetail, setAnnTableDetail] = useState<TableDetail | null>(null);
  const [annQuery, setAnnQuery] = useState("AI-native retrieval with dense vectors");
  const [annK, setAnnK] = useState(3);
  const [annMinScore, setAnnMinScore] = useState(0.25);
  /** dense = full f32 cosine (default); binary_sign = legacy compact Hamming. */
  const [annQuantization, setAnnQuantization] = useState<"dense" | "binary_sign">("dense");
  const [annResult, setAnnResult] = useState<SqlResult | null>(null);
  const [modelsNote, setModelsNote] = useState("");

  // Chat (endpoint config persisted in localStorage on Save)
  const [chatCfg, setChatCfg] = useState<ChatConfig>(() => loadChatConfig());
  const [chatInput, setChatInput] = useState("");
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);

  // MCP
  const [mcp, setMcp] = useState<McpStatus | null>(null);
  const [mcpSnippet, setMcpSnippet] = useState<Record<string, unknown> | null>(null);
  const [mcpPort, setMcpPort] = useState(7337);

  // Open form - direct folder or multi-client server
  const [connectMode, setConnectMode] = useState<"direct" | "server">("direct");
  const [path, setPath] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [serverUrl, setServerUrl] = useState("http://127.0.0.1:8453");
  const [serverToken, setServerToken] = useState("");

  useEffect(() => {
    appInfo().then(setInfo).catch(() => undefined);
    listEmbeddingModels()
      .then((m) => setModelsNote(m.note))
      .catch(() => undefined);
    mcpStatus()
      .then(setMcp)
      .catch(() => undefined);
    // Durable flag (app config dir) - not just webview localStorage.
    getDemoUsed()
      .then((used) => {
        setDemoUsedState(used);
        if (used) {
          try {
            localStorage.setItem("mongreldb-viewer.demo-used", "1");
          } catch {
            /* ignore */
          }
        } else {
          // Migrate older localStorage-only flag into durable storage.
          try {
            if (localStorage.getItem("mongreldb-viewer.demo-used") === "1") {
              setDemoUsedState(true);
              void setDemoUsed(true);
            }
          } catch {
            /* ignore */
          }
        }
      })
      .catch(() => {
        try {
          setDemoUsedState(localStorage.getItem("mongreldb-viewer.demo-used") === "1");
        } catch {
          /* ignore */
        }
      });
  }, []);

  const refresh = useCallback(async () => {
    const ov = await getOverview();
    setOverview(ov);
    if (!selectedTable && ov.tables[0]) {
      setSelectedTable(ov.tables[0].name);
      setAnnTable(ov.tables[0].name);
    }
    try {
      setGraph(await getConstellation());
    } catch {
      /* optional */
    }
  }, [selectedTable]);

  useEffect(() => {
    if (!selectedTable || !overview) {
      setTableDetail(null);
      return;
    }
    getTable(selectedTable)
      .then(setTableDetail)
      .catch((e) => setError(String(e)));
  }, [selectedTable, overview]);

  // Keep ANN text-column picker aligned with the chosen table's real columns.
  useEffect(() => {
    if (!annTable || !overview) {
      setAnnTableDetail(null);
      return;
    }
    let cancelled = false;
    getTable(annTable)
      .then((d) => {
        if (cancelled) return;
        setAnnTableDetail(d);
        setAnnTextCol((cur) => preferTextColumn(d.columns, cur));
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [annTable, overview]);

  const withBusy = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    setOk(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const pickDir = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") setPath(selected);
  };

  const applyConnected = async (ov: DatabaseOverview) => {
    setOverview(ov);
    setSelectedTable(ov.tables[0]?.name ?? "");
    setAnnTable(ov.tables[0]?.name ?? "");
    setConnectedAt(Date.now());
    if (ov.connectionMode === "server") {
      setRecents(
        pushRecent({
          mode: "server",
          label: ov.displayLabel || ov.path,
          serverUrl: ov.path,
        }),
      );
    } else {
      setRecents(
        pushRecent({
          mode: "direct",
          label: ov.displayLabel || ov.path,
          path: ov.path,
        }),
      );
    }
    try {
      setGraph(await getConstellation());
    } catch {
      setGraph(null);
    }
    try {
      setInsights(await getInsights());
    } catch {
      setInsights(null);
    }
    setOk(null);
    setView("deck");
  };

  // Optional auto-open for docs/screenshots (VITE_AUTO_OPEN_DB). Not a product default.
  useEffect(() => {
    const autoPath = import.meta.env.VITE_AUTO_OPEN_DB as string | undefined;
    if (!autoPath?.trim()) return;
    let cancelled = false;
    void (async () => {
      setBusy(true);
      setError(null);
      try {
        const ov = await openDatabase({
          path: autoPath.trim(),
          createIfMissing: false,
        });
        if (!cancelled) await applyConnected(ov);
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setBusy(false);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- one-shot screenshot/docs helper
  }, []);

  // Auto-dismiss success toasts
  useEffect(() => {
    if (!ok) return;
    const t = window.setTimeout(() => setOk(null), 4500);
    return () => window.clearTimeout(t);
  }, [ok]);

  // Scroll the main content to the top whenever a banner appears
  useEffect(() => {
    if (!error && !ok) return;
    contentRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  }, [error, ok]);

  // Global shortcuts
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const typing =
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable);
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setPaletteOpen((v) => !v);
        return;
      }
      if (e.key === "?" && !typing && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        setShowHelp((v) => !v);
        return;
      }
      if (typing) return;
      // About is available even without a connection.
      if (e.key === "8" || e.key === "0") {
        setAboutSub("about");
        setView("about");
        return;
      }
      if (e.key >= "1" && e.key <= "7" && overview) {
        const map: View[] = [
          "deck",
          "constellation",
          "table",
          "sql",
          "ann",
          "agent",
          "mcp",
        ];
        setView(map[Number(e.key) - 1]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [overview]);

  const onOpen = () =>
    withBusy(async () => {
      if (connectMode === "server") {
        if (!serverUrl.trim()) throw new Error("Enter the mongreldb-server URL");
        const ov = await openServer({
          url: serverUrl.trim(),
          bearerToken: serverToken || undefined,
          username: username || undefined,
          password: password || undefined,
        });
        await applyConnected(ov);
        return;
      }
      if (!path.trim()) throw new Error("Choose a database directory");
      const ov = await openDatabase({
        path: path.trim(),
        username: username || undefined,
        password: password || undefined,
        passphrase: passphrase || undefined,
        createIfMissing: false,
      });
      await applyConnected(ov);
    });

  const onCreateDemo = () =>
    withBusy(async () => {
      let demoPath = path.trim();
      if (!demoPath) {
        const selected = await open({ directory: true, multiple: false });
        if (typeof selected !== "string") throw new Error("Pick an empty folder for the demo DB");
        demoPath = selected;
        setPath(demoPath);
      }
      const ov = await createDemo(demoPath, true);
      try {
        localStorage.setItem("mongreldb-viewer.demo-used", "1");
      } catch {
        /* ignore */
      }
      try {
        await setDemoUsed(true);
      } catch {
        /* backend also sets this on success */
      }
      setDemoUsedState(true);
      await applyConnected(ov);
    });

  const onClose = () =>
    withBusy(async () => {
      await closeDatabase();
      setOverview(null);
      setInsights(null);
      setGraph(null);
      setTableDetail(null);
      setSqlResult(null);
      setAnnResult(null);
      setSelectedTable("");
      setOk(null);
      setView("deck");
    });

  const runSql = () =>
    withBusy(async () => {
      const res = await executeSql(sql);
      setSqlResult(res);
      setSqlHistory((h) => [sql, ...h.filter((x) => x !== sql)].slice(0, 12));
      setOk(`${res.statementKind} · ${res.rowCount} rows · ${res.elapsedMs} ms`);
      await refresh().catch(() => undefined);
      getInsights()
        .then(setInsights)
        .catch(() => undefined);
    });

  const runSqlText = (text: string) => {
    setSql(text);
    setView("sql");
    void withBusy(async () => {
      const res = await executeSql(text);
      setSqlResult(res);
      setSqlHistory((h) => [text, ...h.filter((x) => x !== text)].slice(0, 12));
      setOk(`${res.statementKind} · ${res.rowCount} rows · ${res.elapsedMs} ms`);
    });
  };

  const paletteActions: PaletteAction[] = useMemo(() => {
    const acts: PaletteAction[] = [];
    acts.push({
      id: "view-home",
      group: "Navigate",
      label: overview ? "Overview" : "Home / connect",
      hint: "helmet",
      run: () => setView("deck"),
    });
    acts.push({
      id: "view-about",
      group: "Navigate",
      label: "About",
      run: () => {
        setAboutSub("about");
        setView("about");
      },
    });
    if (!overview) {
      acts.push({
        id: "create-demo",
        group: "Connect",
        label: "Create demo database",
        hint: "direct",
        run: () => {
          setConnectMode("direct");
          setView("deck");
          setOk("Use Welcome → Create demo DB with an empty folder path.");
        },
      });
      return acts;
    }
    const views: { id: View; label: string }[] = [
      { id: "constellation", label: "Schema map" },
      { id: "table", label: "Table browser" },
      { id: "sql", label: "SQL console" },
      { id: "ann", label: "Vector search" },
      { id: "agent", label: "Chat agent" },
      { id: "mcp", label: "MCP bridge" },
    ];
    for (const v of views) {
      acts.push({
        id: `view-${v.id}`,
        group: "Navigate",
        label: v.label,
        run: () => setView(v.id),
      });
    }
    for (const t of overview.tables) {
      acts.push({
        id: `table-${t.name}`,
        group: "Tables",
        label: t.name,
        hint: `${t.rowCount} rows`,
        run: () => {
          setSelectedTable(t.name);
          setView("table");
        },
      });
      acts.push({
        id: `sql-${t.name}`,
        group: "SQL",
        label: `SELECT * FROM ${t.name}`,
        run: () => runSqlText(`SELECT * FROM ${t.name} LIMIT 25`),
      });
    }
    for (const q of insights?.suggestedQueries ?? []) {
      acts.push({
        id: `recipe-${q.sql}`,
        group: "Recipes",
        label: q.title,
        hint: q.category,
        run: () => runSqlText(q.sql),
      });
    }
    if (selectedTable) {
      acts.push({
        id: `reindex-${selectedTable}`,
        group: "Maintenance",
        label: `REINDEX ${selectedTable}`,
        hint: "analyze + compact + GC",
        run: () => onReindexTable(selectedTable),
      });
    }
    acts.push({
      id: "reindex-all",
      group: "Maintenance",
      label: "REINDEX all tables",
      hint: "database-wide maintenance",
      run: () => onReindexAll(),
    });
    acts.push({
      id: "disconnect",
      group: "Session",
      label: "Disconnect",
      run: () => setDisconnectOpen(true),
    });
    return acts;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [overview, insights, selectedTable]);

  const onInstallAnn = (opts?: { reembed?: boolean; rebuild?: boolean }) =>
    withBusy(async () => {
      const table = annTable || selectedTable;
      const tableMeta = overview?.tables.find((t) => t.name === table);
      const rebuild = !!opts?.rebuild;
      // ANN is durable in the DB schema - skip install if already active
      // (unless re-embed or rebuild).
      if (tableMeta?.hasAnn && !opts?.reembed && !rebuild) {
        setOk(
          `ANN already active on ${table} (${tableMeta.embeddingDims[0] ?? 384}-d). Persists with the database. Use Rebuild to change quantization.`,
        );
        return;
      }
      const textCol = annTextCol.trim();
      if (opts?.reembed && !textCol) {
        setError("Pick a text column that exists on this table to re-embed from.");
        return;
      }
      if (textCol && annTableDetail) {
        const okCol = annTableDetail.columns.some((c) => c.name === textCol);
        if (!okCol) {
          setError(
            `Column \`${textCol}\` is not on \`${table}\`. Choose one of: ${annTableDetail.columns
              .map((c) => c.name)
              .join(", ")}.`,
          );
          return;
        }
      }
      // Rebuild always needs local model only if also re-embedding.
      const willEmbed =
        !!textCol && (rebuild || opts?.reembed || !tableMeta?.hasAnn);
      if (willEmbed) {
        await ensureLocalEmbeddings();
      }
      const res = await installDenseAnn({
        table,
        embeddingColumn: "embedding",
        dimension: 384,
        // Re-embed when: first install, explicit re-embed, or rebuild with a text col selected.
        sourceTextColumn: willEmbed ? textCol || undefined : undefined,
        backfillLimit: 5000,
        quantization: annQuantization,
        rebuild,
      });
      setOk(res.message);
      await refresh();
      if (selectedTable) setTableDetail(await getTable(selectedTable));
      if (annTable) setAnnTableDetail(await getTable(annTable));
    });

  const afterReindex = async () => {
    await refresh();
    if (selectedTable) {
      try {
        setTableDetail(await getTable(selectedTable));
      } catch {
        /* optional */
      }
    }
    if (annTable) {
      try {
        setAnnTableDetail(await getTable(annTable));
      } catch {
        /* optional */
      }
    }
  };

  const onReindexTable = (table: string) =>
    withBusy(async () => {
      const target = table.trim();
      if (!target) {
        setError("Pick a table before REINDEX table.");
        return;
      }
      const res = await reindexDatabase(target);
      setOk(res.message);
      await afterReindex();
    });

  const onReindexAll = () =>
    withBusy(async () => {
      const res = await reindexDatabase();
      setOk(res.message);
      await afterReindex();
    });

  const onSemantic = () =>
    withBusy(async () => {
      const table = annTable || selectedTable;
      const tableMeta = overview?.tables.find((t) => t.name === table);
      if (!tableMeta?.hasAnn) {
        setError(
          `Table \`${table}\` has no ANN index yet. Install ANN first (Dense f32 cosine by default; pick a text column that exists on that table).`,
        );
        return;
      }
      await ensureLocalEmbeddings();
      const k = Math.max(1, Math.min(100, Math.floor(annK) || 3));
      const res = await semanticSearch({
        table,
        embeddingColumn: "embedding",
        query: annQuery,
        k,
        exactRerank: true,
        // Floor weak cosine matches so "banana" does not pad out the full top-k.
        minScore: annMinScore > 0 ? annMinScore : undefined,
      });
      setAnnResult(res);
      setOk(
        `Top-${k} on \`${table}\` · minScore ${annMinScore || "off"} · ${res.rowCount} hits · ${res.elapsedMs} ms`,
      );
    });

  const onChat = () =>
    withBusy(async () => {
      if (!chatCfg.apiKey.trim()) {
        setError("Enter an API key and click Save before chatting.");
        return;
      }
      if (!chatInput.trim()) return;
      const next: ChatMessage[] = [
        ...chatMessages,
        { role: "user", content: chatInput.trim() },
      ];
      setChatMessages(next);
      setChatInput("");
      const res = await chatCompletion(next, chatCfg);
      setChatMessages(res.messages);
      setOk(`Agent round-trip via ${res.model} · tools: ${res.toolTraces.length}`);
    });

  const onStartMcp = () =>
    withBusy(async () => {
      const st = await startMcp("http", "127.0.0.1", mcpPort);
      setMcp(st);
      setMcpSnippet(await mcpConfigSnippet());
      setOk(`MCP live at ${st.endpoint}`);
    });

  const onStopMcp = () =>
    withBusy(async () => {
      setMcp(await stopMcp());
      setOk("MCP stopped");
    });

  const title = useMemo(() => {
    if (!overview && view !== "about") {
      return ["MongrelDB Viewer", "Open a local root or connect to mongreldb-server"];
    }
    switch (view) {
      case "deck":
        return ["MongrelDB Deck", "Insights, tables, and capabilities for the open database"];
      case "constellation":
        return ["Schema map", "Tables, columns, and indexes"];
      case "table":
        return ["Table", "Columns, indexes, and a sample of rows"];
      case "sql":
        return ["SQL", "Run queries against the open database"];
      case "ann":
        return ["Vector search", "Install and try semantic search"];
      case "agent":
        return ["Agent Chat", "Talk to the open database with your API key"];
      case "mcp":
        return ["MCP", "Connect external tools to this database"];
      case "about":
        if (aboutSub === "licenses") {
          return ["Licenses", "Bundled license and attribution documents"];
        }
        if (aboutSub === "credits") {
          return ["Credits", "Runtime libraries and Cargo crates"];
        }
        return ["About", "MongrelDB Viewer product information"];
      default:
        return ["MongrelDB Viewer", ""];
    }
  }, [view, aboutSub, overview]);

  const uptime =
    connectedAt && overview
      ? Math.max(0, Math.floor((Date.now() - connectedAt) / 1000))
      : null;

  return (
    <>
      <div className="scanline" />
      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        actions={paletteActions}
      />
      {showHelp && (
        <div className="palette-backdrop" onClick={() => setShowHelp(false)}>
          <div className="help-card" onClick={(e) => e.stopPropagation()}>
            <h2>Keyboard shortcuts</h2>
            <ul>
              <li>
                <kbd>{paletteShortcutParts().mod}</kbd>+
                <kbd>{paletteShortcutParts().key}</kbd> command palette
              </li>
              <li>
                <kbd>1</kbd>-<kbd>7</kbd> switch views (when not typing)
              </li>
              <li>
                <kbd>Ctrl</kbd>/<kbd>⌘</kbd>+<kbd>Enter</kbd> run SQL
              </li>
              <li>
                <kbd>?</kbd> this help
              </li>
            </ul>
            <button type="button" className="btn primary" onClick={() => setShowHelp(false)}>
              Got it
            </button>
          </div>
        </div>
      )}
      <div className="app-shell">
        <aside className="rail">
          <button
            type="button"
            className={`brand-mark${view === "deck" ? " active" : ""}`}
            title={
              overview
                ? "Overview — database insights"
                : "MongrelDB Viewer — home"
            }
            aria-label="Overview"
            onClick={() => setView("deck")}
          >
            <img src="/helmet-48.png" alt="" width={36} height={36} draggable={false} />
          </button>
          {overview &&
            NAV.map((n) => (
              <button
                key={n.id}
                type="button"
                className={`rail-btn ${view === n.id ? "active" : ""}`}
                onClick={() => setView(n.id)}
                title={n.label}
              >
                <span className="icon">{n.icon}</span>
                <span className="label">{n.label}</span>
              </button>
            ))}
          <div className="rail-spacer" />
          {overview && (
            <button
              type="button"
              className="rail-btn"
              onClick={() => withBusy(refresh)}
              title="Refresh overview and insights"
            >
              <span className="icon">↺</span>
              <span className="label">Sync</span>
            </button>
          )}
          <button
            type="button"
            className={`rail-btn ${view === "about" ? "active" : ""}`}
            onClick={() => {
              setAboutSub("about");
              setView("about");
            }}
            title="About"
          >
            <span className="icon">ℹ</span>
            <span className="label">About</span>
          </button>
        </aside>

        <div className="main">
          <header className="topbar">
            <div>
              <h1>{title[0]}</h1>
              <div className="subtitle">{title[1]}</div>
            </div>
            <div className="topbar-actions">
              {overview ? (
                <>
                  <button
                    type="button"
                    className="btn ghost"
                    onClick={() => setPaletteOpen(true)}
                    title={`Command palette (${paletteShortcutLabel()})`}
                  >
                    {paletteShortcutLabel()}
                  </button>
                  <span
                    className={`mode-badge ${overview.connectionMode === "server" ? "server" : "direct"}`}
                    title={
                      overview.connectionMode === "server"
                        ? "Connected via mongreldb-server (multi-client)"
                        : "Direct embedded open (single process)"
                    }
                  >
                    {overview.connectionMode === "server" ? "Server" : "Direct"}
                  </span>
                  <button
                    type="button"
                    className="pill live path-disconnect-btn"
                    title="Click to Disconnect"
                    disabled={busy}
                    onClick={() => setDisconnectOpen(true)}
                  >
                    <span className="dot" />
                    <span className="path-disconnect-label">
                      {overview.displayLabel || overview.path}
                    </span>
                  </button>
                </>
              ) : (
                <span className="pill warn">
                  <span className="dot" />
                  not connected
                </span>
              )}
              {mcp?.running && (
                <span className="pill live">
                  <span className="dot" />
                  MCP {mcp.endpoint}
                </span>
              )}
            </div>
          </header>

          {disconnectOpen && overview && (
            <div
              className="palette-backdrop"
              role="presentation"
              onClick={() => !busy && setDisconnectOpen(false)}
            >
              <div
                className="disconnect-dialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="disconnect-title"
                onClick={(e) => e.stopPropagation()}
              >
                <div className="disconnect-dialog-halo" aria-hidden />
                <img
                  className="disconnect-dialog-icon"
                  src="/helmet-64.png"
                  alt=""
                  width={56}
                  height={56}
                  draggable={false}
                />
                <h2 id="disconnect-title">Disconnect?</h2>
                <p className="disconnect-dialog-body">
                  This will release the connection.
                </p>
                <div className="disconnect-dialog-actions">
                  <button
                    type="button"
                    className="btn ghost soft-ring-btn"
                    disabled={busy}
                    onClick={() => setDisconnectOpen(false)}
                  >
                    Stay connected
                  </button>
                  <button
                    type="button"
                    className="btn danger"
                    disabled={busy}
                    onClick={() => {
                      setDisconnectOpen(false);
                      onClose();
                    }}
                  >
                    Disconnect
                  </button>
                </div>
              </div>
            </div>
          )}

          <main
            className={`content${view === "constellation" && overview ? " fill-page" : ""}`}
            ref={contentRef}
          >
            {error && (
              <div className="error-banner" role="alert">
                {error}
              </div>
            )}
            {ok && (
              <div className="ok-banner" role="status">
                {ok}
              </div>
            )}

            {view === "about" ? (
              <AboutHub sub={aboutSub} onNavigate={setAboutSub} />
            ) : !overview ? (
              <Welcome
                connectMode={connectMode}
                setConnectMode={setConnectMode}
                path={path}
                setPath={setPath}
                serverUrl={serverUrl}
                setServerUrl={setServerUrl}
                serverToken={serverToken}
                setServerToken={setServerToken}
                username={username}
                setUsername={setUsername}
                password={password}
                setPassword={setPassword}
                passphrase={passphrase}
                setPassphrase={setPassphrase}
                pickDir={pickDir}
                onOpen={onOpen}
                onCreateDemo={onCreateDemo}
                showCreateDemo={!demoUsed}
                busy={busy}
                info={info}
                recents={recents}
                onRecent={(r) => {
                  if (r.mode === "direct" && r.path) {
                    setConnectMode("direct");
                    setPath(r.path);
                    void withBusy(async () => {
                      const ov = await openDatabase({
                        path: r.path!,
                        createIfMissing: false,
                      });
                      await applyConnected(ov);
                    });
                  } else if (r.mode === "server" && r.serverUrl) {
                    setConnectMode("server");
                    setServerUrl(r.serverUrl);
                    void withBusy(async () => {
                      const ov = await openServer({ url: r.serverUrl! });
                      await applyConnected(ov);
                    });
                  }
                }}
              />
            ) : (
              <>
                {view === "deck" && (
                  <Deck
                    overview={overview}
                    insights={insights}
                    uptimeSec={uptime}
                    onSelectTable={(t) => {
                      setSelectedTable(t);
                      setView("table");
                    }}
                    onRunSql={runSqlText}
                    onNavigate={(v) => setView(v)}
                  />
                )}
                {view === "constellation" && (
                  <div className="constellation-page">
                    <div className="panel">
                      <div className="panel-header">
                        <h2>Schema map</h2>
                        <span className="muted">Pan, zoom, and click tables</span>
                      </div>
                      <div className="panel-body">
                        <Constellation
                          graph={graph}
                          onSelectTable={(t) => {
                            setSelectedTable(t);
                            setView("table");
                          }}
                        />
                      </div>
                    </div>
                  </div>
                )}
                {view === "table" && (
                  <div className="stack">
                    <TableView
                      overview={overview}
                      selectedTable={selectedTable}
                      setSelectedTable={setSelectedTable}
                      detail={tableDetail}
                      busy={busy}
                      onSample={() =>
                        runSqlText(`SELECT * FROM ${selectedTable} LIMIT 50`)
                      }
                      onReindexTable={() => onReindexTable(selectedTable)}
                      onReindexDatabase={onReindexAll}
                    />
                    <TableBrowser
                      table={selectedTable}
                      detail={tableDetail}
                      onOpenSql={(q) => {
                        setSql(q);
                        setView("sql");
                      }}
                    />
                  </div>
                )}
                {view === "sql" && (
                  <SqlWorkbench
                    sql={sql}
                    setSql={setSql}
                    runSql={runSql}
                    result={sqlResult}
                    busy={busy}
                    tables={overview.tables.map((t) => t.name)}
                    suggestions={insights?.suggestedQueries ?? []}
                    history={sqlHistory}
                  />
                )}
                {view === "ann" && (
                  <AnnView
                    overview={overview}
                    annTable={annTable}
                    setAnnTable={setAnnTable}
                    annTextCol={annTextCol}
                    setAnnTextCol={setAnnTextCol}
                    annTableDetail={annTableDetail}
                    annQuery={annQuery}
                    setAnnQuery={setAnnQuery}
                    annK={annK}
                    setAnnK={setAnnK}
                    annMinScore={annMinScore}
                    setAnnMinScore={setAnnMinScore}
                    annQuantization={annQuantization}
                    setAnnQuantization={setAnnQuantization}
                    onInstallAnn={onInstallAnn}
                    onSemantic={onSemantic}
                    result={annResult}
                    busy={busy}
                    modelsNote={modelsNote}
                  />
                )}
                {view === "agent" && (
                  <AgentView
                    chatCfg={chatCfg}
                    setChatCfg={setChatCfg}
                    chatInput={chatInput}
                    setChatInput={setChatInput}
                    chatMessages={chatMessages}
                    onChat={onChat}
                    onSave={() =>
                      withBusy(async () => {
                        saveChatConfig(chatCfg);
                        const r = await probeChat(chatCfg);
                        setOk(
                          `Saved endpoint · probe: ${typeof r === "object" ? JSON.stringify(r) : String(r)}`,
                        );
                      })
                    }
                    busy={busy}
                  />
                )}
                {view === "mcp" && (
                  <McpView
                    mcp={mcp}
                    mcpPort={mcpPort}
                    setMcpPort={setMcpPort}
                    onStart={onStartMcp}
                    onStop={onStopMcp}
                    snippet={mcpSnippet}
                    busy={busy}
                  />
                )}
              </>
            )}
          </main>
        </div>
      </div>
    </>
  );
}

function Welcome(props: {
  connectMode: "direct" | "server";
  setConnectMode: (m: "direct" | "server") => void;
  path: string;
  setPath: (v: string) => void;
  serverUrl: string;
  setServerUrl: (v: string) => void;
  serverToken: string;
  setServerToken: (v: string) => void;
  username: string;
  setUsername: (v: string) => void;
  password: string;
  setPassword: (v: string) => void;
  passphrase: string;
  setPassphrase: (v: string) => void;
  pickDir: () => void;
  onOpen: () => void;
  onCreateDemo: () => void;
  showCreateDemo: boolean;
  busy: boolean;
  info: Record<string, unknown> | null;
  recents: RecentConnection[];
  onRecent: (r: RecentConnection) => void;
}) {
  const embed =
    props.info?.defaultEmbedding &&
    typeof props.info.defaultEmbedding === "object"
      ? (props.info.defaultEmbedding as {
          model?: string;
          dimension?: number;
        })
      : null;

  return (
    <div className="empty-state">
      <div className="hero">
        <h1>MongrelDB Viewer</h1>
        <p className="hero-lead">
          MongrelDB is an encrypted, performant database for apps & AI.
        </p>
        <ul className="hero-points">
          <li>
            <strong>Direct</strong> - open a data folder (one exclusive client)
          </li>
          <li>
            <strong>Server</strong> - connect to mongreldb-server (many clients)
          </li>
          <li>Explore schema, SQL, vectors, chat, and MCP from one place</li>
        </ul>
        {props.recents.length > 0 && (
          <div className="recent-strip">
            <span className="muted">Recent</span>
            {props.recents.slice(0, 5).map((r, i) => (
              <button
                key={`${r.mode}-${r.label}-${i}`}
                type="button"
                className="chip-btn"
                onClick={() => props.onRecent(r)}
                title={r.path || r.serverUrl}
              >
                {r.mode === "server" ? "↗ " : "📁 "}
                {r.label.length > 36 ? `${r.label.slice(0, 34)}…` : r.label}
              </button>
            ))}
          </div>
        )}
        <div className="panel welcome-panel">
          <div className="panel-body">
            <div className="conn-tabs">
              <button
                type="button"
                className={`conn-tab ${props.connectMode === "direct" ? "active" : ""}`}
                onClick={() => props.setConnectMode("direct")}
              >
                Direct folder
              </button>
              <button
                type="button"
                className={`conn-tab ${props.connectMode === "server" ? "active" : ""}`}
                onClick={() => props.setConnectMode("server")}
              >
                mongreldb-server
              </button>
            </div>

            {props.connectMode === "direct" ? (
              <>
                <div className="field">
                  <label>Database directory</label>
                  <div className="row">
                    <input
                      style={{ flex: 1 }}
                      value={props.path}
                      onChange={(e) => props.setPath(e.target.value)}
                      placeholder="/path/to/mongreldb-root"
                    />
                    <button type="button" className="btn" onClick={props.pickDir}>
                      Browse
                    </button>
                  </div>
                </div>
                <div className="field">
                  <label>Encryption passphrase (optional)</label>
                  <input
                    type="password"
                    value={props.passphrase}
                    onChange={(e) => props.setPassphrase(e.target.value)}
                  />
                </div>
              </>
            ) : (
              <>
                <div className="field">
                  <label>Server URL</label>
                  <input
                    value={props.serverUrl}
                    onChange={(e) => props.setServerUrl(e.target.value)}
                    placeholder="http://127.0.0.1:8453"
                  />
                </div>
                <div className="field">
                  <label>Bearer token (optional)</label>
                  <input
                    type="password"
                    value={props.serverToken}
                    onChange={(e) => props.setServerToken(e.target.value)}
                    placeholder="if the daemon uses --auth-token"
                  />
                </div>
              </>
            )}

            <div className="grid-2" style={{ gap: 10 }}>
              <div className="field">
                <label>Username (optional)</label>
                <input value={props.username} onChange={(e) => props.setUsername(e.target.value)} />
              </div>
              <div className="field">
                <label>Password</label>
                <input
                  type="password"
                  value={props.password}
                  onChange={(e) => props.setPassword(e.target.value)}
                />
              </div>
            </div>

            <div className="welcome-actions">
              <button type="button" className="btn primary" disabled={props.busy} onClick={props.onOpen}>
                {props.connectMode === "server" ? "Connect to server" : "Open database"}
              </button>
              {props.connectMode === "direct" && props.showCreateDemo && (
                <button type="button" className="btn magenta" disabled={props.busy} onClick={props.onCreateDemo}>
                  Create demo DB
                </button>
              )}
            </div>
            <div className="welcome-meta">
              {props.connectMode === "direct" ? (
                <p>
                  Direct mode opens the folder exclusively in this app - same idea as
                  SQLite, but a directory. Disconnect when you are done.
                </p>
              ) : (
                <p>
                  Server mode talks to a running <strong>mongreldb-server</strong> over HTTP
                  so other clients can stay connected too. Start the daemon with{" "}
                  <code>mongreldb-server ./data 8453</code>.
                </p>
              )}
              {embed?.model && (
                <p>
                  Optional semantic search uses <strong>{embed.model}</strong>
                  {embed.dimension ? ` (${embed.dimension} dimensions)` : ""}.
                </p>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function Deck({
  overview,
  insights,
  uptimeSec,
  onSelectTable,
  onRunSql,
  onNavigate,
}: {
  overview: DatabaseOverview;
  insights: DbInsights | null;
  uptimeSec: number | null;
  onSelectTable: (t: string) => void;
  onRunSql: (sql: string) => void;
  onNavigate: (v: View) => void;
}) {
  const annTables = overview.tables.filter((t) => t.hasAnn).length;
  const totalRows = overview.tables.reduce((a, t) => a + Number(t.rowCount), 0);
  const totalIndexes = overview.tables.reduce((a, t) => a + Number(t.indexCount), 0);
  const caps = {
    bitmap: overview.tables.filter((t) => t.hasBitmap).length,
    range: overview.tables.filter((t) => t.hasLearnedRange).length,
    fm: overview.tables.filter((t) => t.hasFm).length,
    ann: annTables,
    sparse: overview.tables.filter((t) => t.hasSparse).length,
    minhash: overview.tables.filter((t) => t.hasMinhash).length,
  };
  const capMax = Math.max(1, ...Object.values(caps));
  const isServer = overview.connectionMode === "server";
  const label = overview.displayLabel || overview.path;

  return (
    <div className="stack">
      <section className="deck-hero" aria-label="Database overview">
        <div className="deck-hero-halo" aria-hidden />
        <img
          className="deck-hero-icon"
          src="/helmet-64.png"
          alt=""
          width={72}
          height={72}
          draggable={false}
        />
        <div className="deck-hero-text">
          <div className="deck-hero-kicker">Overview</div>
          <h2 title={overview.path}>{label}</h2>
          <p>
            {isServer ? "Connected via mongreldb-server" : "Direct embedded open"} ·{" "}
            {overview.tableCount} table{overview.tableCount === 1 ? "" : "s"} ·{" "}
            {totalRows.toLocaleString()} rows
            {uptimeSec != null ? ` · session ${formatUptime(uptimeSec)}` : ""}
          </p>
          <div className="about-pills">
            <span className="about-pill mono">engine {overview.engineVersion}</span>
            {overview.queryVersion ? (
              <span className="about-pill mono">query {overview.queryVersion}</span>
            ) : null}
            {overview.gitSha ? (
              <span className="about-pill mono" title={overview.gitSha}>
                {overview.gitSha.slice(0, 12)}
              </span>
            ) : null}
            {annTables > 0 ? (
              <span className="about-pill accent">{annTables} ANN-ready</span>
            ) : (
              <span className="about-pill">no ANN yet</span>
            )}
          </div>
        </div>
        <div className="deck-hero-actions">
          <button type="button" className="btn ghost" onClick={() => onNavigate("constellation")}>
            Schema map
          </button>
          <button type="button" className="btn ghost" onClick={() => onNavigate("sql")}>
            SQL
          </button>
          <button type="button" className="btn ghost" onClick={() => onNavigate("ann")}>
            Vector search
          </button>
        </div>
      </section>

      <div className="grid-3">
        <div className="stat-card">
          <div className="k">Tables</div>
          <div className="v">{overview.tableCount}</div>
          <div className="s">{isServer ? "via mongreldb-server" : "direct open"}</div>
        </div>
        <div className="stat-card">
          <div className="k">Total rows</div>
          <div className="v">{totalRows.toLocaleString()}</div>
          <div className="s">across all tables</div>
        </div>
        <div className="stat-card">
          <div className="k">Secondary indexes</div>
          <div className="v">{totalIndexes}</div>
          <div className="s">
            {annTables > 0
              ? `${annTables} with dense/binary ANN`
              : uptimeSec != null
                ? `session ${formatUptime(uptimeSec)}`
                : "Bitmap · Range · Text · ANN · Sparse · MinHash"}
          </div>
        </div>
      </div>

      <div className="panel">
        <div className="panel-header">
          <h2>Index radar</h2>
          <span className="muted">tables offering each index kind</span>
        </div>
        <div className="panel-body">
          <div className="deck-radar">
            {(
              [
                ["Bitmap", caps.bitmap, "bitmap"],
                ["Range", caps.range, "pgm"],
                ["Text", caps.fm, "fm"],
                ["ANN", caps.ann, "ann"],
                ["Sparse", caps.sparse, "sparse"],
                ["MinHash", caps.minhash, "minhash"],
              ] as const
            ).map(([label, n, chip]) => (
              <div className="deck-radar-item" key={label}>
                <div className="row" style={{ justifyContent: "space-between" }}>
                  <span className={`chip ${chip}`}>{label}</span>
                  <strong>{n}</strong>
                </div>
                <div className="bar">
                  <i style={{ width: `${(n / capMax) * 100}%` }} />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {insights && insights.cards.length > 0 && (
        <div className="panel">
          <div className="panel-header">
            <h2>Insights</h2>
            <span className="muted">click a card to run its query</span>
          </div>
          <div className="panel-body">
            <div className="insight-grid">
              {insights.cards.map((c, i) => (
                <button
                  key={`${c.title}-${i}`}
                  type="button"
                  className={`insight-card accent-${c.accent}`}
                  disabled={!c.sql}
                  onClick={() => c.sql && onRunSql(c.sql)}
                >
                  <div className="k">{c.title}</div>
                  <div className="v">{c.value}</div>
                  <div className="s">{c.detail}</div>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {insights && insights.suggestedQueries.length > 0 && (
        <div className="panel">
          <div className="panel-header">
            <h2>Try these</h2>
            <span className="muted">recipes for this catalog</span>
          </div>
          <div className="panel-body">
            <div className="chip-row">
              {insights.suggestedQueries.slice(0, 10).map((q) => (
                <button
                  key={q.sql}
                  type="button"
                  className="chip-btn"
                  onClick={() => onRunSql(q.sql)}
                  title={q.description || q.sql}
                >
                  {q.title}
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      <div className="panel">
        <div className="panel-header">
          <h2>Tables</h2>
          <span className={`mode-badge ${isServer ? "server" : "direct"}`}>
            {isServer ? "Server" : "Direct"}
          </span>
        </div>
        <div className="panel-body">
          <div className="table-wrap">
            <table className="data">
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Rows</th>
                  <th>Cols</th>
                  <th>Indexes</th>
                  <th>Capabilities</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {overview.tables.map((t) => (
                  <tr key={t.name}>
                    <td
                      style={{ cursor: "pointer", fontWeight: 600 }}
                      onClick={() => onSelectTable(t.name)}
                    >
                      {t.name}
                    </td>
                    <td>{t.rowCount}</td>
                    <td>{t.columnCount}</td>
                    <td>{t.indexCount}</td>
                    <td>
                      <div className="row" style={{ gap: 4 }}>
                        {t.hasBitmap && (
                          <span className="chip bitmap" title="Roaring bitmap equality">
                            Bitmap
                          </span>
                        )}
                        {t.hasLearnedRange && (
                          <span className="chip pgm" title="PGM learned range">
                            Range
                          </span>
                        )}
                        {t.hasFm && (
                          <span className="chip fm" title="FM-index substring">
                            Text
                          </span>
                        )}
                        {t.hasAnn && (
                          <span className="chip ann" title="HNSW ANN">
                            ANN
                          </span>
                        )}
                        {t.hasSparse && <span className="chip sparse">Sparse</span>}
                        {t.hasMinhash && <span className="chip minhash">MinHash</span>}
                      </div>
                    </td>
                    <td>
                      <button
                        type="button"
                        className="btn ghost"
                        onClick={() => onRunSql(`SELECT * FROM ${t.name} LIMIT 25`)}
                      >
                        Preview
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="footer-meta">
            MongrelDB {overview.engineVersion}
            {overview.gitSha ? ` · ${overview.gitSha.slice(0, 12)}` : ""} · press{" "}
            <kbd>?</kbd> for shortcuts · {paletteShortcutLabel()} palette
          </p>
        </div>
      </div>
    </div>
  );
}

function formatUptime(sec: number): string {
  if (sec < 60) return `${sec}s`;
  if (sec < 3600) return `${Math.floor(sec / 60)}m`;
  return `${Math.floor(sec / 3600)}h${Math.floor((sec % 3600) / 60)}m`;
}

function TableView({
  overview,
  selectedTable,
  setSelectedTable,
  detail,
  busy,
  onSample,
  onReindexTable,
  onReindexDatabase,
}: {
  overview: DatabaseOverview;
  selectedTable: string;
  setSelectedTable: (t: string) => void;
  detail: TableDetail | null;
  busy: boolean;
  onSample: () => void;
  onReindexTable: () => void;
  onReindexDatabase: () => void;
}) {
  const radar = detail?.indexRadar;
  const max = Math.max(
    1,
    radar?.bitmap ?? 0,
    radar?.learnedRange ?? 0,
    radar?.fmIndex ?? 0,
    radar?.ann ?? 0,
    radar?.sparse ?? 0,
    radar?.minhash ?? 0,
  );
  return (
    <div className="grid-2">
      <div className="stack">
        <div className="panel">
          <div className="panel-header">
            <h2>Inspector</h2>
            <div className="row" style={{ minWidth: 280 }}>
              <select
                className="control"
                style={{ minWidth: 180 }}
                value={selectedTable}
                onChange={(e) => setSelectedTable(e.target.value)}
              >
                {overview.tables.map((t) => (
                  <option key={t.name} value={t.name}>
                    {t.name}
                  </option>
                ))}
              </select>
              <button type="button" className="btn" onClick={onSample}>
                Sample rows
              </button>
              <button
                type="button"
                className="btn ghost"
                disabled={busy || !selectedTable}
                title="REINDEX this table (analyze + compact + GC)"
                onClick={onReindexTable}
              >
                REINDEX table
              </button>
              <button
                type="button"
                className="btn ghost"
                disabled={busy}
                title="REINDEX entire database"
                onClick={onReindexDatabase}
              >
                REINDEX all
              </button>
            </div>
          </div>
          <div className="panel-body">
            {detail ? (
              <>
                <p className="muted">
                  schema_id={detail.schemaId} · rows={detail.rowCount} · cols=
                  {detail.columns.length}
                </p>
                <div className="table-wrap">
                  <table className="data">
                    <thead>
                      <tr>
                        <th>ID</th>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Flags</th>
                        <th>Embedding source</th>
                      </tr>
                    </thead>
                    <tbody>
                      {detail.columns.map((c) => (
                        <tr key={c.id}>
                          <td>{c.id}</td>
                          <td>{c.name}</td>
                          <td>{c.typeName}</td>
                          <td>{c.flags.join(", ") || "-"}</td>
                          <td className="muted">
                            {c.embeddingSource ||
                              (c.embeddingDim != null ? "supplied_by_application" : "—")}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            ) : (
              <div className="muted">Select a table</div>
            )}
          </div>
        </div>
      </div>
      <div className="stack">
        <div className="panel">
          <div className="panel-header">
            <h2>Index radar</h2>
          </div>
          <div className="panel-body">
            {radar ? (
              <div className="radar">
                {(
                  [
                    ["Bitmap", radar.bitmap],
                    ["LearnedRange", radar.learnedRange],
                    ["FM-index", radar.fmIndex],
                    ["ANN", radar.ann],
                    ["Sparse", radar.sparse],
                    ["MinHash", radar.minhash],
                  ] as const
                ).map(([label, n]) => (
                  <div className="radar-item" key={label}>
                    <div className="row" style={{ justifyContent: "space-between" }}>
                      <span>{label}</span>
                      <strong>{n}</strong>
                    </div>
                    <div className="bar">
                      <i style={{ width: `${(n / max) * 100}%` }} />
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="muted">-</div>
            )}
          </div>
        </div>
        <div className="panel">
          <div className="panel-header">
            <h2>Indexes</h2>
          </div>
          <div className="panel-body">
            {detail?.indexes.length ? (
              <div className="table-wrap">
                <table className="data">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Kind</th>
                      <th>Column</th>
                      <th>Options</th>
                    </tr>
                  </thead>
                  <tbody>
                    {detail.indexes.map((i) => (
                      <tr key={i.name}>
                        <td>{i.name}</td>
                        <td>
                          <span className={`chip ${chipClass(i.kind)}`}>{prettyIndexKind(i.kind)}</span>
                        </td>
                        <td>{i.columnName}</td>
                        <td className="muted" title={i.optionsSummary ?? undefined}>
                          {i.optionsSummary ||
                            (i.ann
                              ? prettyQuantization(i.ann.quantization)
                              : "—")}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="muted">No secondary indexes</div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}


function AnnView(props: {
  overview: DatabaseOverview;
  annTable: string;
  setAnnTable: (v: string) => void;
  annTextCol: string;
  setAnnTextCol: (v: string) => void;
  annTableDetail: TableDetail | null;
  annQuery: string;
  setAnnQuery: (v: string) => void;
  annK: number;
  setAnnK: (v: number) => void;
  annMinScore: number;
  setAnnMinScore: (v: number) => void;
  annQuantization: "dense" | "binary_sign";
  setAnnQuantization: (v: "dense" | "binary_sign") => void;
  onInstallAnn: (opts?: { reembed?: boolean; rebuild?: boolean }) => void;
  onSemantic: () => void;
  result: SqlResult | null;
  busy: boolean;
  modelsNote: string;
}) {
  const selected = props.overview.tables.find((t) => t.name === props.annTable);
  const annReady = !!selected?.hasAnn;
  const embDim = selected?.embeddingDims?.[0] ?? 384;
  const activeAnn = annIndexOn(props.annTableDetail);
  const activeQuant = activeAnn?.ann?.quantization;
  const textCols = textColumnOptions(props.annTableDetail?.columns ?? []);
  const hasTextSource = textCols.length > 0;
  const textColValid = !!props.annTextCol && textCols.some((c) => c.name === props.annTextCol);
  const isServer = props.overview.connectionMode === "server";
  const schemaLoaded = props.annTableDetail?.name === props.annTable;
  // Install only when we can actually embed from a real text column on a direct open.
  const canEnable =
    !props.busy && !isServer && !annReady && schemaLoaded && hasTextSource && textColValid;
  const canReembed = !props.busy && !isServer && annReady && textColValid;
  // Rebuild does not require a text column (index-only); re-embed is optional if selected.
  const canRebuild = !props.busy && !isServer && annReady && schemaLoaded;
  const canSearch = !props.busy && annReady;
  const installLabel =
    props.annQuantization === "binary_sign"
      ? "Enable 384-d BinarySign ANN + embed with MiniLM"
      : "Enable 384-d Dense ANN + embed with MiniLM";
  const rebuildLabel =
    props.annQuantization === "binary_sign"
      ? "Rebuild as BinarySign ANN"
      : "Rebuild as Dense ANN";
  const quantChanging =
    !!activeQuant && activeQuant !== props.annQuantization;

  let enableBlockedReason: string | null = null;
  if (isServer) {
    enableBlockedReason =
      "Install ANN needs a direct connection. On a server, run the matching SQL DDL instead.";
  } else if (annReady) {
    enableBlockedReason = null;
  } else if (!schemaLoaded) {
    enableBlockedReason = "Loading table schema…";
  } else if (!hasTextSource) {
    enableBlockedReason = `Table \`${props.annTable}\` has no embeddable text columns (Bytes/JSON/string). ANN + MiniLM needs something to embed.`;
  } else if (!textColValid) {
    enableBlockedReason = "Pick a text column on this table before enabling.";
  }

  return (
    <div className="stack">
      <div className="grid-2">
        <div className="panel">
          <div className="panel-header">
            <h2>{annReady ? "ANN index" : "Install ANN"}</h2>
            {annReady && <span className="status-pill ready">Active</span>}
            {!annReady && schemaLoaded && !hasTextSource && (
              <span className="status-pill blocked">Not eligible</span>
            )}
          </div>
          <div className="panel-body">
            <p className="field-lede">{props.modelsNote}</p>
            <div className="field">
              <label htmlFor="ann-table">Table</label>
              <select
                id="ann-table"
                className="control"
                value={props.annTable}
                onChange={(e) => props.setAnnTable(e.target.value)}
              >
                {props.overview.tables.map((t) => (
                  <option key={t.name} value={t.name}>
                    {t.name}
                    {t.hasAnn
                      ? " · vector ready"
                      : t.name === props.annTable && schemaLoaded && !hasTextSource
                        ? " · not eligible"
                        : ""}
                  </option>
                ))}
              </select>
            </div>

            {annReady ? (
              <div className="ann-status ready" role="status">
                <div className="ann-status-title">
                  {prettyQuantization(activeQuant)} · {embDim}-d HNSW
                </div>
                <p className="ann-status-body">
                  {activeAnn?.optionsSummary
                    ? `Options: ${activeAnn.optionsSummary}. `
                    : null}
                  Index lives in this database — still available after you close
                  the app and reopen the same path. Search on the right is ready.
                  {activeQuant === "binary_sign"
                    ? " BinarySign uses Hamming prefilter; exact cosine rerank still applies when enabled."
                    : " Dense stores full f32 vectors with cosine distance in the graph."}
                </p>
              </div>
            ) : enableBlockedReason && !isServer ? (
              <div className="ann-status blocked" role="status">
                <div className="ann-status-title">Cannot enable on this table</div>
                <p className="ann-status-body">{enableBlockedReason}</p>
              </div>
            ) : null}

            <div className="field">
              <label htmlFor="ann-quant">Quantization</label>
              <span className="field-hint">
                Dense is full f32 cosine (recommended). BinarySign is the legacy
                compact Hamming path.
                {annReady
                  ? " Changing this requires Rebuild (drop + create index)."
                  : null}
              </span>
              <select
                id="ann-quant"
                className="control"
                value={props.annQuantization}
                onChange={(e) =>
                  props.setAnnQuantization(e.target.value as "dense" | "binary_sign")
                }
                disabled={isServer}
              >
                <option value="dense">Dense · f32 cosine (default)</option>
                <option value="binary_sign">BinarySign · compact Hamming (advanced)</option>
              </select>
            </div>

            <div className="field">
              <label htmlFor="ann-text-col">Text column</label>
              <span className="field-hint">
                Embed source on <code>{props.annTable || "this table"}</code>
                {annReady
                  ? " · required for re-embed; optional when rebuilding"
                  : " · install + embed"}
              </span>
              <select
                id="ann-text-col"
                className="control"
                value={textColValid ? props.annTextCol : ""}
                onChange={(e) => props.setAnnTextCol(e.target.value)}
                disabled={!hasTextSource && schemaLoaded}
              >
                <option value="">
                  {hasTextSource ? "- select a text column -" : "- no embeddable columns -"}
                </option>
                {textCols.map((c) => (
                  <option key={c.name} value={c.name}>
                    {c.name} · {c.typeName}
                  </option>
                ))}
              </select>
            </div>

            <div className="field-actions">
              {annReady ? (
                <>
                  <button
                    type="button"
                    className="btn ghost"
                    disabled={!canReembed}
                    onClick={() => props.onInstallAnn({ reembed: true })}
                    title={
                      canReembed
                        ? "Rewrite embedding vectors from the text column"
                        : "Pick a valid text column to re-embed"
                    }
                  >
                    Re-embed from text column
                  </button>
                  <button
                    type="button"
                    className="btn magenta"
                    disabled={!canRebuild}
                    onClick={() => props.onInstallAnn({ rebuild: true })}
                    title={
                      canRebuild
                        ? quantChanging
                          ? `Drop ANN and recreate as ${props.annQuantization}; re-embeds if a text column is selected`
                          : "Drop ANN and recreate with the selected quantization; re-embeds if a text column is selected"
                        : "Rebuild needs a direct connection and an existing ANN"
                    }
                  >
                    {rebuildLabel}
                  </button>
                </>
              ) : (
                <button
                  type="button"
                  className="btn magenta"
                  disabled={!canEnable}
                  onClick={() => props.onInstallAnn()}
                  title={enableBlockedReason ?? "Install ANN and embed rows"}
                >
                  {installLabel}
                </button>
              )}
            </div>
            {annReady && quantChanging && (
              <p className="field-hint" style={{ marginTop: 12 }}>
                Active index is {prettyQuantization(activeQuant)}; rebuild will switch to{" "}
                {prettyQuantization(props.annQuantization)}.
              </p>
            )}
            {enableBlockedReason && !annReady && (
              <p className="field-hint" style={{ marginTop: 12 }}>
                {enableBlockedReason}
              </p>
            )}
          </div>
        </div>
        <div className="panel">
          <div className="panel-header">
            <h2>Semantic search</h2>
          </div>
          <div className="panel-body ann-search-form">
            <p className="field-lede">
              Top-k nearest neighbors with exact cosine rerank — not a keyword filter.
              Weak hits fall away below the min score.
              {activeQuant === "dense"
                ? " Dense ANN also scores with cosine distance inside HNSW."
                : activeQuant === "binary_sign"
                  ? " BinarySign HNSW prefilters with Hamming; rerank restores cosine order."
                  : null}
            </p>
            <div className="field">
              <label htmlFor="ann-query">Query</label>
              <textarea
                id="ann-query"
                className="query-input"
                rows={3}
                placeholder="e.g. dense ANN with exact cosine rerank"
                value={props.annQuery}
                onChange={(e) => props.setAnnQuery(e.target.value)}
              />
            </div>
            <div className="field-row">
              <div className="field">
                <label htmlFor="ann-k">Max hits</label>
                <span className="field-hint">k · 1-100</span>
                <input
                  id="ann-k"
                  type="number"
                  min={1}
                  max={100}
                  step={1}
                  value={props.annK}
                  onChange={(e) => props.setAnnK(Number(e.target.value))}
                />
              </div>
              <div className="field">
                <label htmlFor="ann-min-score">Min score</label>
                <span className="field-hint">cosine · 0 = off</span>
                <input
                  id="ann-min-score"
                  type="number"
                  min={0}
                  max={1}
                  step={0.05}
                  value={props.annMinScore}
                  onChange={(e) => props.setAnnMinScore(Number(e.target.value))}
                />
              </div>
            </div>
            <div className="field-actions">
              <button
                className="btn primary"
                disabled={!canSearch}
                onClick={props.onSemantic}
                title={
                  canSearch
                    ? "Run semantic search on this table"
                    : "Install ANN on this table before searching"
                }
              >
                Search (HNSW + exact rerank)
              </button>
            </div>
            {!annReady && (
              <p className="field-hint" style={{ marginTop: 10 }}>
                Semantic search only runs on tables that already have an ANN
                index (look for “vector ready”).
              </p>
            )}
          </div>
        </div>
      </div>
      {props.result && (
        <div className="panel">
          <div className="panel-header">
            <h2>Hits</h2>
            <span className="muted">
              {props.result.rowCount} · {props.result.elapsedMs} ms
              {props.result.columns.some((c) => c === "exact_score")
                ? " · ranked by exact_score"
                : ""}
            </span>
          </div>
          <div className="panel-body">
            <DataTable columns={props.result.columns} rows={props.result.rows} />
          </div>
        </div>
      )}
    </div>
  );
}

const CHAT_CONFIG_KEY = "mongreldb-viewer.chat-config";

const DEFAULT_CHAT_CONFIG: ChatConfig = {
  baseUrl: "https://api.openai.com/v1",
  apiKey: "",
  model: "gpt-4o-mini",
  systemPrompt: null,
};

function loadChatConfig(): ChatConfig {
  try {
    const raw = localStorage.getItem(CHAT_CONFIG_KEY);
    if (!raw) return { ...DEFAULT_CHAT_CONFIG };
    const parsed = JSON.parse(raw) as Partial<ChatConfig>;
    return {
      baseUrl: typeof parsed.baseUrl === "string" ? parsed.baseUrl : DEFAULT_CHAT_CONFIG.baseUrl,
      apiKey: typeof parsed.apiKey === "string" ? parsed.apiKey : "",
      model: typeof parsed.model === "string" ? parsed.model : DEFAULT_CHAT_CONFIG.model,
      systemPrompt:
        typeof parsed.systemPrompt === "string" || parsed.systemPrompt === null
          ? parsed.systemPrompt ?? null
          : null,
    };
  } catch {
    return { ...DEFAULT_CHAT_CONFIG };
  }
}

function saveChatConfig(cfg: ChatConfig) {
  localStorage.setItem(
    CHAT_CONFIG_KEY,
    JSON.stringify({
      baseUrl: cfg.baseUrl,
      apiKey: cfg.apiKey,
      model: cfg.model,
      systemPrompt: cfg.systemPrompt ?? null,
    }),
  );
}

function AgentView(props: {
  chatCfg: ChatConfig;
  setChatCfg: (c: ChatConfig) => void;
  chatInput: string;
  setChatInput: (s: string) => void;
  chatMessages: ChatMessage[];
  onChat: () => void;
  onSave: () => void;
  busy: boolean;
}) {
  const canSend = !props.busy && !!props.chatCfg.apiKey.trim() && !!props.chatInput.trim();
  return (
    <div className="grid-2">
      <div className="panel">
        <div className="panel-header">
          <h2>Conversation</h2>
        </div>
        <div className="panel-body">
          <div className="chat-log">
            {props.chatMessages.length === 0 && (
              <div className="muted">
                Ask about schema, write SQL, or run semantic search. The agent has the same tools as MCP.
                {!props.chatCfg.apiKey.trim()
                  ? " Enter an API key and click Save before sending."
                  : ""}
              </div>
            )}
            {props.chatMessages.map((m, i) => (
              <div key={i} className={`bubble ${m.role === "user" ? "user" : m.role === "tool" ? "tool" : "assistant"}`}>
                <div className="muted" style={{ fontSize: 10, marginBottom: 4 }}>
                  {m.role}
                  {m.name ? ` · ${m.name}` : ""}
                </div>
                {m.content || (m.toolCalls ? JSON.stringify(m.toolCalls, null, 2) : "")}
              </div>
            ))}
          </div>
          <div className="field" style={{ marginTop: 12 }}>
            <label>Message</label>
            <textarea
              value={props.chatInput}
              onChange={(e) => props.setChatInput(e.target.value)}
              placeholder="What indexes does documents use? Find rows about hybrid retrieval."
              disabled={!props.chatCfg.apiKey.trim()}
            />
          </div>
          <button
            className="btn primary"
            disabled={!canSend}
            onClick={props.onChat}
            title={
              !props.chatCfg.apiKey.trim()
                ? "Enter an API key in the endpoint form and click Save"
                : !props.chatInput.trim()
                  ? "Type a message first"
                  : "Send message"
            }
          >
            Send
          </button>
        </div>
      </div>
      <div className="panel">
        <div className="panel-header">
          <h2>OpenAI-compatible endpoint</h2>
          <button
            className="btn ghost"
            disabled={props.busy}
            onClick={props.onSave}
            title="Save endpoint settings and validate connectivity"
          >
            Save
          </button>
        </div>
        <div className="panel-body">
          <div className="field">
            <label>Base URL</label>
            <input
              value={props.chatCfg.baseUrl}
              onChange={(e) => props.setChatCfg({ ...props.chatCfg, baseUrl: e.target.value })}
              placeholder="https://api.openai.com/v1"
            />
          </div>
          <div className="field">
            <label>API key</label>
            <input
              type="password"
              value={props.chatCfg.apiKey}
              onChange={(e) => props.setChatCfg({ ...props.chatCfg, apiKey: e.target.value })}
            />
          </div>
          <div className="field">
            <label>Model</label>
            <input
              value={props.chatCfg.model}
              onChange={(e) => props.setChatCfg({ ...props.chatCfg, model: e.target.value })}
            />
          </div>
          <p className="muted">
            Works with OpenAI, Azure OpenAI, SpaceXAI, Ollama (`http://127.0.0.1:11434/v1`), and any
            compatible gateway. Tool calls share the MCP tool surface. Click <strong>Save</strong> to
            store these settings for next launch and probe the endpoint.
          </p>
        </div>
      </div>
    </div>
  );
}

function McpView(props: {
  mcp: McpStatus | null;
  mcpPort: number;
  setMcpPort: (n: number) => void;
  onStart: () => void;
  onStop: () => void;
  snippet: Record<string, unknown> | null;
  busy: boolean;
}) {
  return (
    <div className="stack">
      <div className="panel">
        <div className="panel-header">
          <h2>MCP server</h2>
          <div className="row">
            <span className={`pill ${props.mcp?.running ? "live" : ""}`}>
              <span className="dot" />
              {props.mcp?.running ? "running" : "stopped"}
            </span>
          </div>
        </div>
        <div className="panel-body">
          <p className="muted">
            Start an HTTP JSON-RPC MCP endpoint for Claude Desktop, Cursor, or any MCP client.
            You can keep using in-app Agent chat at the same time - both share the open database.
          </p>
          <div className="field" style={{ maxWidth: 200 }}>
            <label>Port</label>
            <input
              type="number"
              value={props.mcpPort}
              onChange={(e) => props.setMcpPort(Number(e.target.value) || 7337)}
            />
          </div>
          <div className="row">
            <button className="btn primary" disabled={props.busy} onClick={props.onStart}>
              Start MCP
            </button>
            <button className="btn ghost" disabled={props.busy} onClick={props.onStop}>
              Stop
            </button>
          </div>
          {props.mcp?.endpoint && (
            <p className="mono" style={{ marginTop: 12 }}>
              {props.mcp.endpoint}
            </p>
          )}
          {props.mcp?.tools?.length ? (
            <div className="row" style={{ marginTop: 12 }}>
              {props.mcp.tools.map((t) => (
                <span className="chip" key={t}>
                  {t}
                </span>
              ))}
            </div>
          ) : null}
        </div>
      </div>
      {props.snippet && (
        <div className="panel">
          <div className="panel-header">
            <h2>Client config</h2>
          </div>
          <div className="panel-body">
            <div className="code-block">{JSON.stringify(props.snippet, null, 2)}</div>
            <p className="muted" style={{ marginTop: 10 }}>
              Stdio alternative: <code>MONGRELDB_VIEWER_PATH=/path/to/db mongreldb-viewer --mcp-stdio</code>
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
