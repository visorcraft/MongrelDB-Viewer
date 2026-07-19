import { useEffect, useMemo, useState } from "react";
import { paletteShortcutLabel } from "../lib/platform";

export type PaletteAction = {
  id: string;
  label: string;
  hint?: string;
  group: string;
  run: () => void;
};

type Props = {
  open: boolean;
  onClose: () => void;
  actions: PaletteAction[];
};

export function CommandPalette({ open, onClose, actions }: Props) {
  const [q, setQ] = useState("");
  const [idx, setIdx] = useState(0);

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (!needle) return actions;
    return actions.filter(
      (a) =>
        a.label.toLowerCase().includes(needle) ||
        a.hint?.toLowerCase().includes(needle) ||
        a.group.toLowerCase().includes(needle),
    );
  }, [actions, q]);

  useEffect(() => {
    if (open) {
      setQ("");
      setIdx(0);
    }
  }, [open]);

  useEffect(() => {
    setIdx(0);
  }, [q]);

  if (!open) return null;

  const run = (a: PaletteAction) => {
    onClose();
    a.run();
  };

  return (
    <div
      className="palette-backdrop"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
      }}
    >
      <div
        className="palette"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Command palette"
      >
        <input
          autoFocus
          className="palette-input"
          placeholder="Jump to a view, table, or query…  (Esc to close)"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setIdx((i) => Math.min(filtered.length - 1, i + 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setIdx((i) => Math.max(0, i - 1));
            } else if (e.key === "Enter" && filtered[idx]) {
              e.preventDefault();
              run(filtered[idx]);
            } else if (e.key === "Escape") {
              onClose();
            }
          }}
        />
        <div className="palette-list">
          {filtered.length === 0 && <div className="palette-empty">No matches</div>}
          {filtered.map((a, i) => (
            <button
              key={a.id}
              type="button"
              className={`palette-item ${i === idx ? "active" : ""}`}
              onMouseEnter={() => setIdx(i)}
              onClick={() => run(a)}
            >
              <span className="palette-group">{a.group}</span>
              <span className="palette-label">{a.label}</span>
              {a.hint && <span className="palette-hint">{a.hint}</span>}
            </button>
          ))}
        </div>
        <div className="palette-footer muted">
          ↑↓ navigate · Enter run · Esc close · also {paletteShortcutLabel()}
        </div>
      </div>
    </div>
  );
}
