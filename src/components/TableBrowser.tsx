import { useEffect, useState } from "react";
import { executeSql, type SqlResult, type TableDetail } from "../lib/api";
import { DataTable } from "./DataTable";

type Props = {
  table: string;
  detail: TableDetail | null;
  onOpenSql: (sql: string) => void;
};

export function TableBrowser({ table, detail, onOpenSql }: Props) {
  const [limit, setLimit] = useState(50);
  const [result, setResult] = useState<SqlResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [cols, setCols] = useState<string>("*");

  const load = async () => {
    if (!table) return;
    setBusy(true);
    setError(null);
    try {
      const projection =
        cols === "*"
          ? "*"
          : cols
              .split(",")
              .map((c) => c.trim())
              .filter(Boolean)
              .join(", ") || "*";
      const res = await executeSql(
        `SELECT ${projection} FROM ${table} LIMIT ${limit}`,
        limit,
      );
      setResult(res);
    } catch (e) {
      setError(String(e));
      setResult(null);
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    setCols("*");
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [table]);

  const nonEmbed =
    detail?.columns.filter((c) => !c.typeName.toLowerCase().includes("embedding")) ??
    [];

  return (
    <div className="panel">
      <div className="panel-header">
        <h2>Live rows</h2>
        <div className="row">
          <select
            className="control"
            style={{ width: 110 }}
            value={limit}
            onChange={(e) => setLimit(Number(e.target.value))}
          >
            {[25, 50, 100, 250, 500].map((n) => (
              <option key={n} value={n}>
                {n} rows
              </option>
            ))}
          </select>
          <button type="button" className="btn" disabled={busy} onClick={() => void load()}>
            {busy ? "Loading…" : "Refresh"}
          </button>
          <button
            type="button"
            className="btn ghost"
            onClick={() =>
              onOpenSql(
                `SELECT ${cols === "*" ? "*" : cols} FROM ${table} LIMIT ${limit}`,
              )
            }
          >
            Open in SQL
          </button>
        </div>
      </div>
      <div className="panel-body">
        {nonEmbed.length > 0 && (
          <div className="chip-row" style={{ marginBottom: 12 }}>
            <button
              type="button"
              className={`chip-btn ${cols === "*" ? "on" : ""}`}
              onClick={() => setCols("*")}
            >
              All columns
            </button>
            <button
              type="button"
              className="chip-btn"
              onClick={() => {
                setCols(nonEmbed.map((c) => c.name).join(", "));
              }}
            >
              Hide embeddings
            </button>
            {nonEmbed.slice(0, 8).map((c) => (
              <span key={c.id} className="chip bitmap" title={c.typeName}>
                {c.name}
              </span>
            ))}
          </div>
        )}
        {error && <div className="error-banner">{error}</div>}
        {result && (
          <>
            <div className="muted" style={{ marginBottom: 8 }}>
              Showing {result.rowCount}
              {result.truncated ? "+" : ""} rows · {result.elapsedMs} ms
            </div>
            <DataTable columns={result.columns} rows={result.rows} maxHeight={360} />
          </>
        )}
        {!result && !error && !busy && (
          <div className="muted">Pick a table to browse rows.</div>
        )}
      </div>
    </div>
  );
}
