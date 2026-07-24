# Compatibility fixtures

Frozen MongrelDB roots used to catch **cross-version open regressions**
(WAL layout, catalog/schema decode, Kit sidecar, SQL against recovered tables).

## `sample-demo-v0.64.5.tar.gz`

| Field | Value |
| ----- | ----- |
| Contents | Viewer `create_demo` root (`with_ann = false`) |
| Written by | `mongreldb-*` **0.64.5** |
| Top-level dir | `sample-demo/` |
| Also includes | `kit_schema.json`, `FIXTURE_META.json` |

The archive is intentionally **version-pinned in the filename**. When you bump
the engine crates (for example 0.64.5 → 0.64.8), **keep this file** so the test
`frozen_sample_demo_remains_usable_on_current_engine` still opens a root
produced by an older train.

Only regenerate when the demo schema itself changes on purpose (new tables,
column renames, intentional seed changes):

```sh
# from repository root
scripts/gen-compat-fixture.sh
```

After a deliberate schema change you should:

1. Run the regen script.
2. Rename the archive to match the **new** writing train (and update
   `COMPAT_FIXTURE_*` constants in `src/db/session.rs`).
3. Commit the new archive **and** keep older ones if you still care about
   multi-hop compatibility (optional; at least one prior-train fixture is
   required).

## Why a tarball?

Root `.gitignore` excludes local DBs and `*.wal`. Packaging the root as
`.tar.gz` keeps the fixture committed, bit-stable, and free of accidental
partial checkouts.
