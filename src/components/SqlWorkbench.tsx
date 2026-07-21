import { useMemo, useState } from "react";
import type { SqlResult, SuggestedQuery } from "../lib/api";
import { DataTable } from "./DataTable";

type Props = {
  sql: string;
  setSql: (s: string) => void;
  runSql: () => void;
  result: SqlResult | null;
  busy: boolean;
  tables: string[];
  suggestions: SuggestedQuery[];
  history: string[];
};

export function SqlWorkbench({
  sql,
  setSql,
  runSql,
  result,
  busy,
  tables,
  suggestions,
  history,
}: Props) {
  const [filter, setFilter] = useState("all");
  const cats = useMemo(() => {
    const s = new Set(suggestions.map((q) => q.category));
    return ["all", ...Array.from(s)];
  }, [suggestions]);

  const visible = suggestions.filter(
    (q) => filter === "all" || q.category === filter,
  );

  return (
    <div className="grid-2 sql-workbench">
      <div className="stack">
        <div className="panel">
          <div className="panel-header">
            <h2>SQL</h2>
            <div className="row">
              <button
                type="button"
                className="btn ghost"
                onClick={() =>
                  setSql(
                    tables[0]
                      ? `SELECT * FROM ${tables[0]} LIMIT 25`
                      : "SELECT 1",
                  )
                }
              >
                Sample
              </button>
              <button
                type="button"
                className="btn ghost"
                disabled={!sql}
                onClick={() => navigator.clipboard?.writeText(sql)}
                title="Copy SQL"
              >
                Copy
              </button>
              <button type="button" className="btn primary" disabled={busy} onClick={runSql}>
                Run ⌘↵
              </button>
            </div>
          </div>
          <div className="panel-body">
            <textarea
              className="mono sql-editor"
              value={sql}
              onChange={(e) => setSql(e.target.value)}
              onKeyDown={(e) => {
                if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
                  e.preventDefault();
                  runSql();
                }
              }}
              spellCheck={false}
              placeholder="Write SQL… Ctrl/Cmd+Enter to run"
            />
            {history.length > 0 && (
              <div className="sql-history">
                <div className="muted" style={{ marginBottom: 6 }}>
                  Recent
                </div>
                <div className="chip-row">
                  {history.slice(0, 6).map((h, i) => (
                    <button
                      key={i}
                      type="button"
                      className="chip-btn"
                      title={h}
                      onClick={() => setSql(h)}
                    >
                      {h.length > 42 ? `${h.slice(0, 40)}…` : h}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>

        {result && (
          <div className="panel">
            <div className="panel-header">
              <h2>Result</h2>
              <span className="muted">
                {result.statementKind} · {result.rowCount}
                {result.truncated ? "+" : ""} rows · {result.elapsedMs} ms
              </span>
            </div>
            <div className="panel-body">
              <div className="row" style={{ marginBottom: 10 }}>
                <button
                  type="button"
                  className="btn ghost"
                  onClick={() => copyCsv(result)}
                >
                  Copy CSV
                </button>
              </div>
              <DataTable columns={result.columns} rows={result.rows} maxHeight={480} />
            </div>
          </div>
        )}
      </div>

      <div className="panel">
        <div className="panel-header">
          <h2>Recipes</h2>
        </div>
        <div className="panel-body">
          <div className="chip-row" style={{ marginBottom: 12 }}>
            {cats.map((c) => (
              <button
                key={c}
                type="button"
                className={`chip-btn ${filter === c ? "on" : ""}`}
                onClick={() => setFilter(c)}
              >
                {c}
              </button>
            ))}
          </div>
          <div className="recipe-list">
            {visible.map((q) => (
              <button
                key={q.sql}
                type="button"
                className="recipe-card"
                onClick={() => setSql(q.sql)}
              >
                <div className="recipe-title">{q.title}</div>
                <div className="recipe-desc muted">{q.description}</div>
                <code className="recipe-sql">{q.sql}</code>
              </button>
            ))}
            {visible.length === 0 && (
              <div className="muted">Connect a database to unlock recipes.</div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function copyCsv(result: SqlResult) {
  const esc = (v: unknown) => {
    const s = v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
    if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
    return s;
  };
  const lines = [
    result.columns.map(esc).join(","),
    ...result.rows.map((r) => r.map(esc).join(",")),
  ];
  navigator.clipboard?.writeText(lines.join("\n"));
}
