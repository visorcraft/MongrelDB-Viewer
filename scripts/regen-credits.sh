#!/usr/bin/env bash
# Regenerate legal/crates.json + legal/third-party.md after dependency changes.
set -euo pipefail
cd "$(dirname "$0")/../src-tauri"
python3 - <<'PY'
import json, subprocess, pathlib
from collections import Counter
meta = json.loads(subprocess.check_output(
    ["cargo","metadata","--format-version","1","--manifest-path","Cargo.toml"], text=True))
pkgs_by_id = {p["id"]: p for p in meta["packages"]}
used = []
for n in meta["resolve"]["nodes"]:
    p = pkgs_by_id.get(n["id"])
    if not p or p["name"] == "mongreldb-viewer":
        continue
    used.append({
        "name": p["name"],
        "version": p["version"],
        "license": p.get("license") or "",
        "repository": p.get("repository") or p.get("homepage") or "",
    })
seen=set(); rows=[]
for r in sorted(used, key=lambda x: (x["name"].lower(), x["version"])):
    k=(r["name"], r["version"])
    if k in seen: continue
    seen.add(k); rows.append(r)
pathlib.Path("legal").mkdir(exist_ok=True)
pathlib.Path("legal/crates.json").write_text(json.dumps(rows, indent=2)+"\n")
print(f"wrote legal/crates.json ({len(rows)} crates)")
print("top licenses", Counter(r["license"] or "?" for r in rows).most_common(8))
PY
cargo about generate about.hbs -o legal/third-party.md
echo "wrote legal/third-party.md ($(wc -l < legal/third-party.md) lines)"
