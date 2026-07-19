export type RecentConnection = {
  mode: "direct" | "server";
  label: string;
  path?: string;
  serverUrl?: string;
  at: number;
};

const KEY = "mongreldb-viewer.recents.v1";
const MAX = 8;

export function loadRecents(): RecentConnection[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as RecentConnection[];
    return Array.isArray(parsed) ? parsed.slice(0, MAX) : [];
  } catch {
    return [];
  }
}

export function pushRecent(entry: Omit<RecentConnection, "at">) {
  const next: RecentConnection = { ...entry, at: Date.now() };
  const prev = loadRecents().filter((r) => {
    if (entry.mode === "direct") return !(r.mode === "direct" && r.path === entry.path);
    return !(r.mode === "server" && r.serverUrl === entry.serverUrl);
  });
  const list = [next, ...prev].slice(0, MAX);
  localStorage.setItem(KEY, JSON.stringify(list));
  return list;
}

export function clearRecents() {
  localStorage.removeItem(KEY);
}
