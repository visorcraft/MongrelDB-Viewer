import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ConstellationGraph } from "../lib/api";

type Props = {
  graph: ConstellationGraph | null;
  onSelectTable?: (name: string) => void;
};

const KIND_COLOR: Record<string, string> = {
  database: "#5ef0e0",
  table: "#ff5ad8",
  column: "#b7c3d9",
  embedding: "#ffd166",
  index: "#a89bff",
};

type Pt = { x: number; y: number };

function layout(graph: ConstellationGraph): { positions: Map<string, Pt>; bounds: { minX: number; minY: number; maxX: number; maxY: number } } {
  const positions = new Map<string, Pt>();
  const tables = graph.nodes.filter((n) => n.kind === "table");
  const dbNode = graph.nodes.find((n) => n.kind === "database");

  const cx = 0;
  const cy = 0;
  if (dbNode) positions.set(dbNode.id, { x: cx, y: cy });

  // Spread tables farther so labels, satellites, and FK arcs fit.
  const tableR = Math.max(260, 110 + tables.length * 55);
  tables.forEach((t, i) => {
    const angle = (i / Math.max(tables.length, 1)) * Math.PI * 2 - Math.PI / 2;
    const tr = tableR + (i % 2) * 42;
    const tx = cx + Math.cos(angle) * tr;
    const ty = cy + Math.sin(angle) * tr;
    positions.set(t.id, { x: tx, y: ty });

    const cols = graph.nodes.filter((n) => n.id.startsWith(`col:${t.label}:`));
    const colR = 100 + Math.min(cols.length, 14) * 5;
    cols.forEach((c, j) => {
      const a = angle - 0.6 + (j / Math.max(cols.length - 1, 1)) * 1.2;
      positions.set(c.id, {
        x: tx + Math.cos(a) * colR,
        y: ty + Math.sin(a) * colR,
      });
    });

    const idxs = graph.nodes.filter((n) => n.id.startsWith(`idx:${t.label}:`));
    const idxR = colR + 58;
    idxs.forEach((idx, j) => {
      const a = angle + 0.12 + j * 0.2;
      positions.set(idx.id, {
        x: tx + Math.cos(a) * idxR,
        y: ty + Math.sin(a) * idxR,
      });
    });
  });

  graph.nodes.forEach((n, i) => {
    if (!positions.has(n.id)) {
      const a = (i / graph.nodes.length) * Math.PI * 2;
      positions.set(n.id, { x: Math.cos(a) * (tableR + 80), y: Math.sin(a) * (tableR + 80) });
    }
  });

  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (const p of positions.values()) {
    minX = Math.min(minX, p.x);
    minY = Math.min(minY, p.y);
    maxX = Math.max(maxX, p.x);
    maxY = Math.max(maxY, p.y);
  }
  // Padding for labels
  const pad = 80;
  return {
    positions,
    bounds: { minX: minX - pad, minY: minY - pad, maxX: maxX + pad, maxY: maxY + pad },
  };
}

export function Constellation({ graph, onSelectTable }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const sizeRef = useRef({ w: 900, h: 560 });
  const [size, setSize] = useState({ w: 900, h: 560 });
  const [view, setView] = useState({ x: 0, y: 0, k: 1 });
  const [hover, setHover] = useState<string | null>(null);
  const drag = useRef<{ active: boolean; sx: number; sy: number; ox: number; oy: number } | null>(
    null,
  );
  /** Only auto-fit when the schema graph identity changes - never on pan/zoom/resize noise. */
  const fittedGraphKey = useRef<string | null>(null);

  const graphKey = useMemo(
    () => (graph ? graph.nodes.map((n) => n.id).join("\0") : ""),
    [graph],
  );

  const { positions, bounds } = useMemo(() => {
    if (!graph || graph.nodes.length === 0) {
      return {
        positions: new Map<string, Pt>(),
        bounds: { minX: -200, minY: -200, maxX: 200, maxY: 200 },
      };
    }
    return layout(graph);
  }, [graph]);

  const boundsRef = useRef(bounds);
  boundsRef.current = bounds;

  const fit = useCallback(() => {
    const b = boundsRef.current;
    const { w: sw, h: sh } = sizeRef.current;
    const w = b.maxX - b.minX || 400;
    const h = b.maxY - b.minY || 400;
    const pad = 40;
    const k = Math.min((sw - pad * 2) / w, (sh - pad * 2) / h, 1.4);
    const cx = (b.minX + b.maxX) / 2;
    const cy = (b.minY + b.maxY) / 2;
    setView({
      k,
      x: sw / 2 - cx * k,
      y: sh / 2 - cy * k,
    });
  }, []);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const r = entries[0]?.contentRect;
      if (!r) return;
      // Fill whatever the flex parent gives us (full remaining main area).
      const next = { w: Math.max(320, r.width), h: Math.max(200, r.height) };
      // Ignore sub-pixel thrash that would otherwise re-trigger layout logic.
      const prev = sizeRef.current;
      if (Math.abs(prev.w - next.w) < 2 && Math.abs(prev.h - next.h) < 2) return;
      sizeRef.current = next;
      setSize(next);
    });
    ro.observe(el);
    const initial = { w: el.clientWidth || 900, h: el.clientHeight || 560 };
    sizeRef.current = initial;
    setSize(initial);
    return () => ro.disconnect();
  }, []);

  // Auto-fit only when the graph content changes (or first size is ready for a new graph).
  useEffect(() => {
    if (!graphKey) return;
    if (fittedGraphKey.current === graphKey) return;
    if (sizeRef.current.w < 40) return;
    fittedGraphKey.current = graphKey;
    fit();
  }, [graphKey, size.w, size.h, fit]);

  if (!graph || graph.nodes.length === 0) {
    return (
      <div className="constellation-shell">
        <div className="constellation" style={{ display: "grid", placeItems: "center" }}>
          <span className="muted">No schema yet - connect to a database first.</span>
        </div>
      </div>
    );
  }

  const onWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12;
    setView((v) => {
      const nextK = Math.min(3.5, Math.max(0.25, v.k * factor));
      const wx = (mx - v.x) / v.k;
      const wy = (my - v.y) / v.k;
      return {
        k: nextK,
        x: mx - wx * nextK,
        y: my - wy * nextK,
      };
    });
  };

  const onPointerDown = (e: React.PointerEvent) => {
    if ((e.target as Element).closest("[data-node]")) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag.current = {
      active: true,
      sx: e.clientX,
      sy: e.clientY,
      ox: view.x,
      oy: view.y,
    };
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (!drag.current?.active) return;
    const d = drag.current;
    setView((v) => ({
      ...v,
      x: d.ox + (e.clientX - d.sx),
      y: d.oy + (e.clientY - d.sy),
    }));
  };
  const onPointerUp = () => {
    if (drag.current) drag.current.active = false;
  };

  return (
    <div className="constellation-shell">
      <div className="constellation-toolbar">
        <span className="muted">
          Scroll to zoom · drag empty space to pan · click a table to open it
        </span>
        <div className="row">
          <button type="button" className="btn ghost" onClick={fit}>
            Fit all
          </button>
          <button
            type="button"
            className="btn ghost"
            onClick={() => setView((v) => ({ ...v, k: Math.min(3.5, v.k * 1.2) }))}
          >
            +
          </button>
          <button
            type="button"
            className="btn ghost"
            onClick={() => setView((v) => ({ ...v, k: Math.max(0.25, v.k / 1.2) }))}
          >
            -
          </button>
        </div>
      </div>
      <div
        ref={containerRef}
        className="constellation"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerUp}
        style={{ touchAction: "none", cursor: drag.current?.active ? "grabbing" : "grab" }}
      >
        <svg width={size.w} height={size.h} role="img" aria-label="Schema constellation">
          <defs>
            <filter id="glow">
              <feGaussianBlur stdDeviation="2.2" result="coloredBlur" />
              <feMerge>
                <feMergeNode in="coloredBlur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>
          <g transform={`translate(${view.x} ${view.y}) scale(${view.k})`}>
            {graph.edges.map((e, i) => {
              const a = positions.get(e.from);
              const b = positions.get(e.to);
              if (!a || !b) return null;
              const isFk = e.kind === "fk" || e.kind === "fk-col";
              const stroke =
                e.kind === "fk"
                  ? "rgba(255,90,216,0.75)"
                  : e.kind === "fk-col"
                    ? "rgba(255,90,216,0.35)"
                    : e.kind === "index"
                      ? "rgba(168,155,255,0.45)"
                      : e.kind === "covers"
                        ? "rgba(255,209,102,0.4)"
                        : "rgba(94,240,224,0.28)";
              return (
                <line
                  key={`${e.from}-${e.to}-${i}`}
                  x1={a.x}
                  y1={a.y}
                  x2={b.x}
                  y2={b.y}
                  stroke={stroke}
                  strokeWidth={
                    e.kind === "fk"
                      ? 2.6 / view.k
                      : e.kind === "owns"
                        ? 2 / view.k
                        : 1.2 / view.k
                  }
                  strokeDasharray={isFk ? `${6 / view.k} ${4 / view.k}` : undefined}
                />
              );
            })}

            {graph.nodes.map((n) => {
              const p = positions.get(n.id)!;
              const color = KIND_COLOR[n.kind] ?? "#b7c3d9";
              const r =
                n.kind === "database" ? 20 : n.kind === "table" ? 14 : n.kind === "index" ? 9 : 7;
              const clickable = n.kind === "table";
              const isHover = hover === n.id;
              return (
                <g
                  key={n.id}
                  data-node={n.id}
                  transform={`translate(${p.x}, ${p.y})`}
                  style={{ cursor: clickable ? "pointer" : "default" }}
                  filter="url(#glow)"
                  onMouseEnter={() => setHover(n.id)}
                  onMouseLeave={() => setHover(null)}
                  onClick={(ev) => {
                    ev.stopPropagation();
                    if (clickable) onSelectTable?.(n.label);
                  }}
                >
                  <circle r={r + (isHover ? 10 : 7)} fill={color} opacity={isHover ? 0.18 : 0.1} />
                  <circle
                    r={r}
                    fill="#0a1018"
                    stroke={color}
                    strokeWidth={(isHover ? 2.4 : 1.8) / Math.max(view.k, 0.5)}
                  />
                  {n.kind === "table" && <circle r={3.5} fill={color} />}
                  <text
                    y={r + 16}
                    textAnchor="middle"
                    fill="#eef3ff"
                    fontSize={n.kind === "table" || n.kind === "database" ? 13 : 11}
                    fontFamily="IBM Plex Mono, monospace"
                    fontWeight={n.kind === "table" ? 600 : 500}
                    style={{ pointerEvents: "none" }}
                  >
                    {n.label.length > 22 ? `${n.label.slice(0, 20)}…` : n.label}
                  </text>
                </g>
              );
            })}
          </g>
        </svg>
      </div>
      {hover && (
        <div className="constellation-hint">
          {(() => {
            const n = graph.nodes.find((x) => x.id === hover);
            if (!n) return null;
            return (
              <>
                <strong>{n.label}</strong>
                <span className="muted"> · {n.kind}</span>
                {n.meta && (
                  <span className="muted">
                    {" "}
                    · {JSON.stringify(n.meta).replace(/[{}"]/g, " ").slice(0, 80)}
                  </span>
                )}
              </>
            );
          })()}
        </div>
      )}
    </div>
  );
}
