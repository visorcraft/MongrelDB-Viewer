type Props = {
  columns: string[];
  rows: unknown[][];
  maxHeight?: number | string;
};

function cell(v: unknown): string {
  if (v === null || v === undefined) return "∅";
  if (typeof v === "object") return JSON.stringify(v);
  return String(v);
}

export function DataTable({ columns, rows, maxHeight = 420 }: Props) {
  if (!columns.length) {
    return <div className="muted">No columns / empty result.</div>;
  }
  return (
    <div className="table-wrap" style={{ maxHeight }}>
      <table className="data">
        <thead>
          <tr>
            {columns.map((c) => (
              <th key={c}>{c}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i}>
              {r.map((v, j) => (
                <td key={j} title={cell(v)}>
                  {cell(v)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
