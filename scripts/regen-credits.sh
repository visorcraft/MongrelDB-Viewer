#!/usr/bin/env bash
# Regenerate legal inventories after dependency changes:
#   - legal/crates.json          (Cargo transitive list for Credits)
#   - legal/third-party.md       (cargo-about full Rust license texts)
#   - legal/npm-packages.json    (npm list for Credits)
#   - legal/npm-third-party.md   (npm license texts for Licenses)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/src-tauri"

python3 - <<'PY'
import json
import os
import re
import subprocess
import pathlib
from collections import Counter, defaultdict

ROOT = pathlib.Path("..").resolve()
LEGAL = pathlib.Path("legal")
LEGAL.mkdir(exist_ok=True)

# ---------------------------------------------------------------------------
# Cargo crates
# ---------------------------------------------------------------------------
meta = json.loads(
    subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--manifest-path", "Cargo.toml"],
        text=True,
    )
)
pkgs_by_id = {p["id"]: p for p in meta["packages"]}
used = []
for n in meta["resolve"]["nodes"]:
    p = pkgs_by_id.get(n["id"])
    if not p or p["name"] == "mongreldb-viewer":
        continue
    used.append(
        {
            "name": p["name"],
            "version": p["version"],
            "license": p.get("license") or "",
            "repository": p.get("repository") or p.get("homepage") or "",
        }
    )
seen = set()
rows = []
for r in sorted(used, key=lambda x: (x["name"].lower(), x["version"])):
    k = (r["name"], r["version"])
    if k in seen:
        continue
    seen.add(k)
    rows.append(r)
(LEGAL / "crates.json").write_text(json.dumps(rows, indent=2) + "\n")
print(f"wrote legal/crates.json ({len(rows)} crates)")
print("top licenses", Counter(r["license"] or "?" for r in rows).most_common(8))

# ---------------------------------------------------------------------------
# npm packages (installed tree + package-lock metadata)
# ---------------------------------------------------------------------------
lock_path = ROOT / "package-lock.json"
pkg_path = ROOT / "package.json"
lock = json.loads(lock_path.read_text()) if lock_path.exists() else {"packages": {}}
root_pkg = json.loads(pkg_path.read_text()) if pkg_path.exists() else {}
direct_runtime = set((root_pkg.get("dependencies") or {}).keys())
direct_dev = set((root_pkg.get("devDependencies") or {}).keys())

LICENSE_NAMES = (
    "LICENSE",
    "LICENSE.md",
    "LICENSE.txt",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "LICENSE_MIT",
    "LICENSE_APACHE-2.0",
    "LICENSE.BSD",
    "LICENCE",
    "LICENCE.md",
    "COPYING",
    "COPYING.md",
)


def normalize_repo(url: str) -> str:
    if not url:
        return ""
    url = url.strip()
    url = re.sub(r"^git\+", "", url)
    url = re.sub(r"^git://", "https://", url)
    url = re.sub(r"^ssh://git@", "https://", url)
    url = re.sub(r"^git@([^:]+):", r"https://\1/", url)
    url = re.sub(r"\.git$", "", url)
    return url


def license_from_pkg(p: dict) -> str:
    lic = p.get("license")
    if isinstance(lic, str) and lic.strip():
        return lic.strip()
    if isinstance(lic, dict):
        t = lic.get("type") or lic.get("name") or ""
        if t:
            return str(t).strip()
    licenses = p.get("licenses")
    if isinstance(licenses, list) and licenses:
        parts = []
        for item in licenses:
            if isinstance(item, str):
                parts.append(item)
            elif isinstance(item, dict):
                parts.append(str(item.get("type") or item.get("name") or ""))
        parts = [x for x in parts if x]
        if parts:
            return " OR ".join(parts)
    return ""


def read_license_texts(pkg_dir: pathlib.Path) -> list[str]:
    texts = []
    for name in LICENSE_NAMES:
        f = pkg_dir / name
        if f.is_file():
            try:
                texts.append(f.read_text(encoding="utf-8", errors="replace").strip())
            except OSError:
                pass
    # dual-license files already covered by LICENSE_MIT / LICENSE_APACHE-2.0
    if not texts:
        for f in sorted(pkg_dir.glob("LICENSE*")):
            if f.is_file():
                try:
                    texts.append(f.read_text(encoding="utf-8", errors="replace").strip())
                except OSError:
                    pass
    return [t for t in texts if t]


def package_name_from_lock_key(key: str) -> str:
    # node_modules/foo, node_modules/@scope/name, nested node_modules/a/node_modules/b
    parts = key.split("node_modules/")
    return parts[-1]


npm_rows = []
license_groups: dict[str, list[dict]] = defaultdict(list)
# Track packages that exist on disk
for key, meta in (lock.get("packages") or {}).items():
    if not key or not key.startswith("node_modules/"):
        continue
    pkg_dir = ROOT / key
    pkg_json = pkg_dir / "package.json"
    if not pkg_json.is_file():
        # Optional platform binaries listed in the lock but not installed here.
        continue
    try:
        p = json.loads(pkg_json.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        continue
    name = p.get("name") or package_name_from_lock_key(key)
    version = p.get("version") or meta.get("version") or ""
    license_expr = license_from_pkg(p)
    repo = ""
    r = p.get("repository")
    if isinstance(r, str):
        repo = r
    elif isinstance(r, dict):
        repo = r.get("url") or ""
    if not repo:
        repo = p.get("homepage") or ""
    repo = normalize_repo(repo)
    is_dev = bool(meta.get("dev"))
    # Direct deps override: package-lock marks some production deps without dev flag.
    if name in direct_runtime:
        role = "runtime"
    elif name in direct_dev:
        role = "dev"
    elif is_dev:
        role = "dev"
    else:
        role = "runtime"
    row = {
        "name": name,
        "version": version,
        "license": license_expr,
        "repository": repo,
        "role": role,
    }
    npm_rows.append(row)
    texts = read_license_texts(pkg_dir)
    if not texts and license_expr:
        texts = [
            f"(Full license text not found in package tarball.)\n"
            f"SPDX / declared license: {license_expr}\n"
            f"Package: {name}@{version}\n"
            f"Repository: {repo or 'n/a'}"
        ]
    elif not texts:
        texts = [
            f"(No license file or SPDX expression found.)\n"
            f"Package: {name}@{version}\n"
            f"Repository: {repo or 'n/a'}"
        ]
    for t in texts:
        license_groups[t].append({"name": name, "version": version, "repository": repo})

# Dedupe npm rows by name@version (prefer runtime role if both)
dedup: dict[tuple[str, str], dict] = {}
for r in sorted(npm_rows, key=lambda x: (0 if x["role"] == "runtime" else 1, x["name"], x["version"])):
    k = (r["name"], r["version"])
    if k not in dedup:
        dedup[k] = r
npm_out = sorted(dedup.values(), key=lambda x: (x["name"].lower(), x["version"]))
(LEGAL / "npm-packages.json").write_text(json.dumps(npm_out, indent=2) + "\n")
print(
    f"wrote legal/npm-packages.json ({len(npm_out)} packages; "
    f"runtime={sum(1 for r in npm_out if r['role']=='runtime')}, "
    f"dev={sum(1 for r in npm_out if r['role']=='dev')})"
)

# npm third-party markdown
lines = [
    "# Frontend (npm) Third-Party Licenses",
    "",
    "This document lists third-party JavaScript packages from `package-lock.json`",
    "that are installed in this workspace (runtime and build tooling), grouped by",
    "license text. MongrelDB Viewer is dual-licensed under MIT OR Apache-2.0;",
    "the packages listed here are included under their stated licenses and we",
    "acknowledge their authors and copyright holders accordingly.",
    "",
    "This file is auto-generated by `scripts/regen-credits.sh`.",
    "Regenerate after npm dependency changes.",
    "",
    "If you have questions about license compliance, please contact",
    "[VisorCraft](https://www.visorcraft.com).",
    "",
    "## Packages in use",
    "",
]
by_lic_expr = Counter(r["license"] or "?" for r in npm_out)
for lic, count in by_lic_expr.most_common():
    lines.append(f"- **{lic}** ({count} package{'s' if count != 1 else ''})")
lines += ["", "---", "", "## License Texts", ""]

# Stable order: largest groups first, then by first package name
grouped = []
for text, users in license_groups.items():
    # dedupe users
    seen_u = set()
    uniq_users = []
    for u in sorted(users, key=lambda x: (x["name"].lower(), x["version"])):
        k = (u["name"], u["version"])
        if k in seen_u:
            continue
        seen_u.add(k)
        uniq_users.append(u)
    # heading from first line or SPDX-ish
    heading = "Custom / package license"
    head = text.splitlines()[0].strip() if text else heading
    if len(head) > 80:
        head = head[:77] + "..."
    if head:
        heading = head
    grouped.append((heading.lower(), heading, uniq_users, text))

grouped.sort(key=lambda g: (-len(g[2]), g[0]))
for _, heading, users, text in grouped:
    lines.append(f"### {heading}")
    lines.append("")
    lines.append("Used by:")
    for u in users:
        if u["repository"]:
            lines.append(f"- [`{u['name']} {u['version']}`]({u['repository']})")
        else:
            lines.append(f"- `{u['name']} {u['version']}`")
    lines.append("")
    lines.append("```")
    lines.append(text)
    lines.append("```")
    lines.append("")
    lines.append("---")
    lines.append("")

(LEGAL / "npm-third-party.md").write_text("\n".join(lines) + "\n")
print(f"wrote legal/npm-third-party.md ({len(lines)} lines, {len(grouped)} license groups)")

# ---------------------------------------------------------------------------
# Keep acknowledgments.md direct-dep versions in sync with lockfiles
# ---------------------------------------------------------------------------
ack_path = LEGAL / "acknowledgments.md"
ack = ack_path.read_text(encoding="utf-8")

# MongrelDB engine crate versions in the acknowledgments table
crate_ver = {r["name"]: r["version"] for r in rows}
crate_lic = {r["name"]: r["license"] for r in rows}
for name in (
    "mongreldb-core",
    "mongreldb-query",
    "mongreldb-client",
    "mongreldb-kit",
):
    if name in crate_ver:
        ack = re.sub(
            rf"(\| `{re.escape(name)}`)(?:\s+[\d.]+)?(\s+\|)",
            rf"\1 {crate_ver[name]}\2",
            ack,
        )

# Rebuild the Frontend (npm) section tables from package.json + resolved versions
npm_by_name = {r["name"]: r for r in npm_out}


def row(pkg: str, role: str) -> str:
    r = npm_by_name.get(pkg)
    if not r:
        return f"| `{pkg}` | ? | {role} | ? |"
    return f"| `{pkg}` | {r['version']} | {role} | {r['license'] or '?'} |"


runtime_pkgs = [
    ("react", "UI"),
    ("react-dom", "UI DOM renderer"),
    ("@tauri-apps/api", "Frontend bridge"),
    ("@tauri-apps/plugin-dialog", "Dialog plugin client"),
    ("@tauri-apps/plugin-opener", "Opener plugin client"),
]
dev_pkgs = [
    ("@tauri-apps/cli", "Tauri CLI"),
    ("vite", "Bundler / dev server"),
    ("@vitejs/plugin-react", "React plugin for Vite"),
    ("typescript", "Type checking"),
    ("@types/react", "Type definitions"),
    ("@types/react-dom", "Type definitions"),
]
sched = npm_by_name.get("scheduler")

frontend_block = [
    "## Frontend (npm)",
    "",
    "Direct packages from root `package.json` (versions resolved in",
    "`package-lock.json`).",
    "",
    "### Runtime (shipped UI)",
    "",
    "| Package | Version | Role | License |",
    "| ------- | ------- | ---- | ------- |",
]
frontend_block += [row(n, role) for n, role in runtime_pkgs]
frontend_block += [
    "",
    "Transitive runtime package used by React:",
    "",
    "| Package | Version | Role | License |",
    "| ------- | ------- | ---- | ------- |",
]
if sched:
    frontend_block.append(
        f"| `scheduler` | {sched['version']} | React scheduling | {sched['license']} |"
    )
else:
    frontend_block.append("| `scheduler` | ? | React scheduling | ? |")
frontend_block += [
    "",
    "### Build tooling (devDependencies)",
    "",
    "| Package | Version | Role | License |",
    "| ------- | ------- | ---- | ------- |",
]
frontend_block += [row(n, role) for n, role in dev_pkgs]
frontend_block += [
    "",
    "The complete installed npm tree (runtime + build tooling), versions, and full",
    "license texts are available in-app under **Licenses → Frontend (npm)** and as a",
    "filterable table under **Credits**.",
    "",
]

new_frontend = "\n".join(frontend_block)
ack = re.sub(
    r"## Frontend \(npm\).*?(?=\n## Contact\n)",
    new_frontend + "\n",
    ack,
    count=1,
    flags=re.S,
)
ack_path.write_text(ack, encoding="utf-8")
print("updated legal/acknowledgments.md direct-dep versions")
PY

cargo about generate about.hbs -o legal/third-party.md
# Normalize CRLF if cargo-about emits them on some hosts
sed -i 's/\r$//' legal/third-party.md
echo "wrote legal/third-party.md ($(wc -l < legal/third-party.md) lines)"
echo "done."
