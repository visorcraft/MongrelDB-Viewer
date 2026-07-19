import { useEffect, useMemo, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  aboutInfo,
  creditsData,
  licenseDocument,
  licenseDocs,
  runtimeLicenseText,
  type AboutInfo,
  type CreditsData,
  type LicenseDocMeta,
  type RuntimeComponent,
} from "../lib/api";

type AboutSub = "about" | "licenses" | "credits";

const FEATURES = [
  {
    icon: "◎",
    title: "Dense ANN search",
    body: "HNSW prefilter with optional exact cosine rerank and min-score floor.",
  },
  {
    icon: "Σ",
    title: "SQL workbench",
    body: "DataFusion SQL against local embeds or mongreldb-server over HTTP.",
  },
  {
    icon: "✦",
    title: "Schema constellation",
    body: "Tables, FKs, and all six index kinds mapped as a pan/zoom graph.",
  },
  {
    icon: "◉",
    title: "Agent + MCP",
    body: "OpenAI-compatible chat tools and a local MCP bridge for the open DB.",
  },
];

export function AboutHub(props: {
  sub: AboutSub;
  onNavigate: (sub: AboutSub) => void;
}) {
  if (props.sub === "licenses") {
    return <LicensesPage onBack={() => props.onNavigate("about")} />;
  }
  if (props.sub === "credits") {
    return <CreditsPage onBack={() => props.onNavigate("about")} />;
  }
  return (
    <AboutPage
      onLicenses={() => props.onNavigate("licenses")}
      onCredits={() => props.onNavigate("credits")}
    />
  );
}

function AboutPage(props: { onLicenses: () => void; onCredits: () => void }) {
  const [info, setInfo] = useState<AboutInfo | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    aboutInfo()
      .then(setInfo)
      .catch((e) => setErr(String(e)));
  }, []);

  const openRepo = () => {
    if (info?.repository) void openUrl(info.repository);
  };

  return (
    <div className="about-page">
      <header className="about-header">
        <h1>About</h1>
        <p className="about-header-sub">
          {info?.tagline ?? "Built on Rust + Tauri 2 + React."}
        </p>
      </header>

      {err && <div className="error-banner">{err}</div>}

      <div className="about-body">
        <section className="about-hero">
          <div className="about-hero-halo" />
          <img
            className="about-hero-icon"
            src="/helmet-64.png"
            alt="MongrelDB Viewer"
            width={96}
            height={96}
            draggable={false}
          />
          <div className="about-hero-text">
            <h2>{info?.appName ?? "MongrelDB Viewer"}</h2>
            <p>{info?.description ?? "Signal Deck for AI-native MongrelDB databases."}</p>
            <div className="about-pills">
              <span className="about-pill accent">v{info?.version ?? "…"}</span>
              <span className="about-pill">{info?.license ?? "MIT OR Apache-2.0"}</span>
              <span className="about-pill">
                {info?.platform ?? "linux"} · Tauri 2
              </span>
              {info?.gitSha && info.gitSha !== "unknown" && (
                <span className="about-pill mono">{info.gitSha}</span>
              )}
              {info?.engineVersion && (
                <span className="about-pill mono">engine {info.engineVersion}</span>
              )}
            </div>
          </div>
        </section>

        <div className="about-section-label">What&apos;s inside</div>
        <div className="about-features">
          {FEATURES.map((f) => (
            <div className="about-feature" key={f.title}>
              <div className="about-feature-icon">{f.icon}</div>
              <div>
                <div className="about-feature-title">{f.title}</div>
                <div className="about-feature-body">{f.body}</div>
              </div>
            </div>
          ))}
        </div>

        <button type="button" className="about-link-card" onClick={openRepo}>
          <img src="/helmet-48.png" alt="" width={40} height={40} draggable={false} />
          <div className="about-link-card-text">
            <div className="about-link-card-title">
              Source, issues, and releases for MongrelDB Viewer
            </div>
            <div className="about-link-card-url">
              github.com/visorcraft/MongrelDB-Viewer
            </div>
          </div>
          <span className="about-link-card-cta">Visit repo →</span>
        </button>

        <section className="about-legal-card">
          <div className="about-legal-title">Licenses &amp; Credits</div>
          <p className="about-legal-body">
            Every direct + transitive Rust crate and npm package,
            acknowledgments, runtime components, and full license texts are
            bundled in the built-in licenses and credits views.
          </p>
          <div className="about-legal-actions">
            <button type="button" className="btn ghost" onClick={props.onLicenses}>
              ☰ Licenses
            </button>
            <button type="button" className="btn ghost" onClick={props.onCredits}>
              ℹ Credits
            </button>
          </div>
        </section>

        <footer className="about-footer">
          Built by VisorCraft · Powered by Rust, Tauri, React, and MongrelDB
        </footer>
      </div>
    </div>
  );
}

function LicensesPage(props: { onBack: () => void }) {
  const [docs, setDocs] = useState<LicenseDocMeta[]>([]);
  const [active, setActive] = useState("app");
  const [body, setBody] = useState("");
  const [filter, setFilter] = useState("");
  const [wrap, setWrap] = useState(false);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    licenseDocs()
      .then((d) => {
        setDocs(d);
        if (d[0]) setActive(d[0].id);
      })
      .catch((e) => setErr(String(e)));
  }, []);

  useEffect(() => {
    if (!active) return;
    setLoading(true);
    setErr(null);
    licenseDocument(active)
      .then(setBody)
      .catch((e) => setErr(String(e)))
      .finally(() => setLoading(false));
  }, [active]);

  const meta = docs.find((d) => d.id === active) ?? docs[0];
  const visible = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    if (!needle) return body;
    return body
      .split("\n")
      .filter((line) => line.toLowerCase().includes(needle))
      .join("\n");
  }, [body, filter]);

  const lineCount = body ? body.split("\n").length : 0;
  const matchCount = filter.trim()
    ? visible
      ? visible.split("\n").length
      : 0
    : lineCount;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(visible || body);
    } catch {
      /* ignore */
    }
  };

  return (
    <div className="about-page licenses-page">
      <header className="about-header row-header">
        <div>
          <button type="button" className="btn ghost sm" onClick={props.onBack}>
            ← About
          </button>
          <h1>Licenses</h1>
          <p className="about-header-sub">
            Bundled license and attribution documents, available without opening a browser.
          </p>
        </div>
        <div className="licenses-toolbar-actions">
          <button type="button" className="btn ghost sm" onClick={copy}>
            Copy
          </button>
        </div>
      </header>

      <div className="licenses-tabs">
        {docs.map((d) => (
          <button
            key={d.id}
            type="button"
            className={`licenses-tab ${active === d.id ? "active" : ""}`}
            onClick={() => setActive(d.id)}
          >
            {d.title}
          </button>
        ))}
      </div>

      <div className="licenses-doc-meta">
        <div>
          <h2>{meta?.title ?? "…"}</h2>
          <p>{meta?.subtitle}</p>
        </div>
        <div className="licenses-linecount">
          {filter.trim() ? `${matchCount} / ${lineCount}` : `${lineCount}`} lines
        </div>
      </div>

      <div className="licenses-filter-row">
        <input
          className="control licenses-filter"
          placeholder="Find by crate, package, license, or phrase…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <label className="licenses-wrap">
          <input
            type="checkbox"
            checked={wrap}
            onChange={(e) => setWrap(e.target.checked)}
          />
          Wrap
        </label>
        <button type="button" className="btn ghost sm" onClick={() => setFilter("")}>
          Clear
        </button>
      </div>

      {err && <div className="error-banner">{err}</div>}
      <pre className={`licenses-body ${wrap ? "wrap" : ""}`}>
        {loading ? "Loading…" : visible || "(no matching lines)"}
      </pre>
    </div>
  );
}

function CreditsPage(props: { onBack: () => void }) {
  const [data, setData] = useState<CreditsData | null>(null);
  const [crateFilter, setCrateFilter] = useState("");
  const [pkgFilter, setPkgFilter] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [licenseDialog, setLicenseDialog] = useState<{
    title: string;
    body: string;
  } | null>(null);

  useEffect(() => {
    creditsData()
      .then(setData)
      .catch((e) => setErr(String(e)));
  }, []);

  const crates = useMemo(() => {
    const rows = data?.crates ?? [];
    const needle = crateFilter.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter(
      (r) =>
        r.name.toLowerCase().includes(needle) ||
        r.version.toLowerCase().includes(needle) ||
        r.license.toLowerCase().includes(needle),
    );
  }, [data, crateFilter]);

  const packages = useMemo(() => {
    const rows = data?.packages ?? [];
    const needle = pkgFilter.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter(
      (r) =>
        r.name.toLowerCase().includes(needle) ||
        r.version.toLowerCase().includes(needle) ||
        r.license.toLowerCase().includes(needle) ||
        r.role.toLowerCase().includes(needle),
    );
  }, [data, pkgFilter]);

  const openRuntimeLicense = async (comp: RuntimeComponent) => {
    if (!comp.spdx.length) {
      if (comp.projectUrl) void openUrl(comp.projectUrl);
      return;
    }
    try {
      const parts: string[] = [];
      for (const id of comp.spdx) {
        const text = await runtimeLicenseText(id);
        parts.push(`===== ${id} =====\n\n${text}`);
      }
      setLicenseDialog({ title: comp.name, body: parts.join("\n\n\n") });
    } catch (e) {
      setErr(String(e));
    }
  };

  return (
    <div className="about-page credits-page">
      <header className="about-header">
        <button type="button" className="btn ghost sm" onClick={props.onBack}>
          ← About
        </button>
        <h1>Credits</h1>
        <p className="about-header-sub">
          {data
            ? `${data.crateCount} Cargo crates · ${data.packageCount} npm packages · ${data.runtimeCount} runtime components`
            : "Loading…"}
        </p>
      </header>

      {err && <div className="error-banner">{err}</div>}

      <section className="credits-runtime">
        <h2>Runtime components</h2>
        <p className="field-hint">
          System libraries the Viewer links against at execution. None are
          bundled - host OS / packagers provide them.
        </p>
        <div className="credits-runtime-list">
          {(data?.runtime ?? []).map((r) => (
            <div className="credits-runtime-row" key={r.name}>
              <div className="credits-runtime-name">
                <div>{r.name}</div>
                <div className="field-hint">{r.notes}</div>
              </div>
              <div className="credits-runtime-license">{r.licenses}</div>
              <div className="credits-runtime-actions">
                <button
                  type="button"
                  className="btn ghost sm"
                  title="View license text"
                  onClick={() => void openRuntimeLicense(r)}
                  disabled={!r.spdx.length}
                >
                  ☰
                </button>
                <button
                  type="button"
                  className="btn ghost sm"
                  title="Open project"
                  onClick={() => r.projectUrl && void openUrl(r.projectUrl)}
                  disabled={!r.projectUrl}
                >
                  ↗
                </button>
              </div>
            </div>
          ))}
        </div>
      </section>

      <div className="about-section-label">npm packages</div>
      <p className="field-hint" style={{ marginTop: 0 }}>
        Installed JavaScript packages from the workspace lockfile (runtime UI
        plus build tooling). Full texts: Licenses → Frontend (npm).
      </p>
      <div className="credits-filter-row">
        <input
          className="control licenses-filter"
          placeholder="Filter by package name, role, or license…"
          value={pkgFilter}
          onChange={(e) => setPkgFilter(e.target.value)}
        />
        <span className="licenses-linecount">
          {packages.length} / {data?.packageCount ?? 0}
        </span>
      </div>

      <div className="credits-table-wrap">
        <table className="credits-table">
          <thead>
            <tr>
              <th>Package</th>
              <th>Version</th>
              <th>Role</th>
              <th>License expression</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {packages.map((p) => (
              <tr key={`${p.name}@${p.version}`}>
                <td className="mono">{p.name}</td>
                <td className="mono muted">{p.version}</td>
                <td>
                  <span className="license-chip">{p.role || "-"}</span>
                </td>
                <td>
                  <span className="license-chip">{p.license || "-"}</span>
                </td>
                <td>
                  {p.repository ? (
                    <button
                      type="button"
                      className="btn ghost sm"
                      title={p.repository}
                      onClick={() => void openUrl(p.repository)}
                    >
                      ↗
                    </button>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="about-section-label">Cargo crates</div>
      <p className="field-hint" style={{ marginTop: 0 }}>
        Every direct and transitive Rust crate. Full texts: Licenses →
        Third-party (Rust).
      </p>
      <div className="credits-filter-row">
        <input
          className="control licenses-filter"
          placeholder="Filter by crate name or license…"
          value={crateFilter}
          onChange={(e) => setCrateFilter(e.target.value)}
        />
        <span className="licenses-linecount">
          {crates.length} / {data?.crateCount ?? 0}
        </span>
      </div>

      <div className="credits-table-wrap">
        <table className="credits-table">
          <thead>
            <tr>
              <th>Crate</th>
              <th>Version</th>
              <th>License expression</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {crates.map((c) => (
              <tr key={`${c.name}@${c.version}`}>
                <td className="mono">{c.name}</td>
                <td className="mono muted">{c.version}</td>
                <td>
                  <span className="license-chip">{c.license || "-"}</span>
                </td>
                <td>
                  {c.repository ? (
                    <button
                      type="button"
                      className="btn ghost sm"
                      title={c.repository}
                      onClick={() => void openUrl(c.repository)}
                    >
                      ↗
                    </button>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {licenseDialog && (
        <div
          className="palette-backdrop"
          onClick={() => setLicenseDialog(null)}
        >
          <div
            className="license-dialog"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="license-dialog-header">
              <h2>{licenseDialog.title}</h2>
              <button
                type="button"
                className="btn ghost sm"
                onClick={() => setLicenseDialog(null)}
              >
                Close
              </button>
            </div>
            <pre className="licenses-body wrap">{licenseDialog.body}</pre>
          </div>
        </div>
      )}
    </div>
  );
}
