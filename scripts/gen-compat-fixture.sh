#!/usr/bin/env bash
# Regenerate the frozen sample-demo compatibility fixture.
#
# Prefer NOT running this on routine mongreldb-* upgrades — the fixture must
# stay written by an older train so open-compat is actually tested. Only regen
# when the viewer demo schema intentionally changes (see tests/fixtures/README.md).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/src-tauri"

if ! command -v tar >/dev/null 2>&1; then
  echo "error: tar is required to pack the fixture archive" >&2
  exit 1
fi

echo "Regenerating tests/fixtures/sample-demo-v0.64.5.tar.gz from current engine..."
echo "(Requires REGEN_COMPAT_FIXTURE=1; refused otherwise.)"
REGEN_COMPAT_FIXTURE=1 cargo test \
  -p mongreldb-viewer \
  regenerate_frozen_compat_fixture \
  -- --ignored --nocapture

ARCHIVE="tests/fixtures/sample-demo-v0.64.5.tar.gz"
if [[ ! -f "$ARCHIVE" ]]; then
  echo "error: archive was not written: $ARCHIVE" >&2
  exit 1
fi

echo "Verifying open-compat test against the new archive..."
cargo test -p mongreldb-viewer frozen_sample_demo_remains_usable_on_current_engine -- --nocapture

echo "Done. If this was a deliberate demo-schema change, update COMPAT_FIXTURE_*"
echo "constants / archive filename to the writing train, then commit:"
echo "  $ARCHIVE"
echo "  tests/fixtures/README.md"
echo "  src/db/session.rs"
