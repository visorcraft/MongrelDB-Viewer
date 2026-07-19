# Contributing to MongrelDB Viewer

Thanks for helping improve MongrelDB Viewer. This document describes how to
propose a change, what we expect from a pull request, and how to run the
local development gate.

If anything here is unclear or out of date, open an issue or a PR.

## Code of conduct

Be kind, be specific, assume good faith. Disagree about the technical details,
not the person. Public reviews stay focused on the diff.

## How to propose a change

MongrelDB Viewer uses a standard **fork → branch → pull request** workflow on
GitHub.

1. **Fork** [`visorcraft/MongrelDB-Viewer`](https://github.com/visorcraft/MongrelDB-Viewer)
   to your GitHub account.
2. **Clone** your fork and add the upstream remote:

   ```sh
   git clone git@github.com:<you>/MongrelDB-Viewer.git
   cd MongrelDB-Viewer
   git remote add upstream https://github.com/visorcraft/MongrelDB-Viewer.git
   ```

3. **Branch** from `main`. Use a descriptive, kebab-case name:
   `fix-semantic-search-projection`, `feature/about-page`, `docs/contributing`.

   ```sh
   git fetch upstream
   git switch -c my-change upstream/main
   ```

4. **Make focused commits.** One logical change per commit. Prefer Conventional
   Commits style when it fits (`fix:`, `feat:`, `docs:`, `chore:`).
5. **Open a pull request** against `main` on `visorcraft/MongrelDB-Viewer`.
   Include:
   - **What** - one paragraph summary of the change.
   - **Why** - bug fix, feature, docs, or polish; link issues when they exist.
   - **How to test** - exact commands a reviewer should run.
   - **Risk** - what might break; what you did not test.

## Project layout

| Path | Role |
| ---- | ---- |
| `src/` | React + TypeScript UI (Vite) |
| `src-tauri/` | Tauri 2 shell, Rust commands, MongrelDB embed |
| `src-tauri/src/db/` | Direct/server connection, ANN, SQL, demo seed |
| `src-tauri/src/mcp/` | MCP HTTP bridge |
| `src-tauri/legal/` | Bundled licenses, credits inventory, acknowledgments |
| `scripts/` | Linux icon install, credits regeneration |
| `assets/` | README / marketing images |
| `public/` | Web assets (favicon, helmet icons) |

MongrelDB engine crates (`mongreldb-core`, `mongreldb-query`, `mongreldb-client`)
come from **crates.io** - do not re-implement storage, WAL, or ANN logic in the
viewer. Prefer engine APIs and SQL surfaces.

## Prerequisites

- Rust **1.88+**
- Node.js **22+**
- Platform Tauri system libraries (see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

## Before you push: local gate

From the repository root:

```sh
npm install
npm run build
cd src-tauri && cargo check && cargo fmt --check
```

When you touch embeddings or ANN paths, also exercise the Vector search UI
against a demo database created from the Welcome screen.

After changing Rust dependencies, regenerate legal inventories:

```sh
scripts/regen-credits.sh
```

Commit the updated `src-tauri/legal/crates.json` and
`src-tauri/legal/third-party.md` with the dependency change.

### Linux taskbar icon (optional)

For Wayland/X11 taskbar icons when running `npm run tauri dev` without a package
install:

```sh
scripts/install-icons-linux.sh
```

## What we look for in a review

- The change does one thing and does it well.
- Behavior that can regress has a clear manual or automated check.
- Direct vs server connection modes are considered when the change touches DB
  access.
- Dense ANN install/search only claims table-scoped behavior; do not imply
  whole-database semantic search unless you implement it.
- UI copy uses ASCII hyphens (`-`), not em dashes.
- No AI/agent attribution in commits, PRs, code comments, or test identifiers.
- License and credits remain accurate when dependencies change.

## Licensing

Contributions are dual-licensed under **MIT OR Apache-2.0**, the same as the
project. By submitting a pull request you agree your contribution may be
distributed under those terms.

## Questions

- Product / UX: open a GitHub issue with screenshots when helpful.
- Engine behavior: also see [MongrelDB](https://github.com/visorcraft/MongrelDB)
  docs and issues for storage/query questions.
