# Contributing to MongrelDB Viewer

Thank you for improving MongrelDB Viewer. Keep changes focused, test the paths
they affect, and document behavior users can observe.

## Code of conduct

Be kind, specific, and technically honest. Assume good faith. Discuss the code
and product behavior, not the person.

Report vulnerabilities privately through [SECURITY.md](SECURITY.md), not a
public issue or pull request.

## Propose a change

Use the standard fork and pull-request flow:

1. Fork
   [`visorcraft/MongrelDB-Viewer`](https://github.com/visorcraft/MongrelDB-Viewer).
2. Clone your fork:

   ```sh
   git clone git@github.com:<you>/MongrelDB-Viewer.git
   cd MongrelDB-Viewer
   git remote add upstream https://github.com/visorcraft/MongrelDB-Viewer.git
   ```

3. Branch from current `main`:

   ```sh
   git fetch upstream
   git switch -c my-change upstream/main
   ```

4. Make focused commits.
5. Push your branch and open a pull request against `main`.

Good branch names are descriptive and kebab-cased:

```text
fix-semantic-search-projection
feature-about-filter
docs-mcp-reference
```

Conventional commit prefixes are welcome when they fit:

```text
fix:
feat:
docs:
test:
chore:
```

## Set up development

Requirements:

- Rust 1.88+,
- Node.js 22+,
- Tauri 2 platform prerequisites,
- Git,
- `tar` for compatibility tests.

Install and launch:

```sh
npm ci
npm run tauri dev
```

Use an empty directory and **Create demo DB** for disposable manual testing.

See [installation](docs/installation.md) for platform dependencies and
[architecture](docs/architecture.md) before changing cross-layer behavior.

## Project layout

| Path | Responsibility |
| --- | --- |
| `src/App.tsx` | Page state, navigation, connection/ANN/Agent/MCP workflows |
| `src/components/` | Focused React surfaces |
| `src/lib/api.ts` | Typed Tauri invoke wrappers |
| `src/lib/recents.ts` | WebView recent-connection persistence |
| `src/styles/global.css` | UI layout and theme |
| `src-tauri/src/commands/` | Tauri command boundary and shared state |
| `src-tauri/src/db/` | Connections, SQL, schema inspection, demo, ANN |
| `src-tauri/src/embeddings/` | Local/internal remote providers |
| `src-tauri/src/chat/` | Chat Completions tool loop |
| `src-tauri/src/mcp/` | Shared tools and HTTP/stdio transports |
| `src-tauri/src/legal.rs` | Bundled legal-document interface |
| `src-tauri/legal/` | Generated inventories and license texts |
| `src-tauri/tests/fixtures/` | Frozen cross-version database root |
| `docs/` | Public product, integration, operations, and architecture guides |
| `scripts/` | Legal, compatibility-fixture, and Linux icon tasks |

Do not duplicate MongrelDB storage, WAL, query, or ANN behavior in Viewer.
Use the official `mongreldb-core`, `mongreldb-query`, `mongreldb-client`, and
`mongreldb-kit` APIs and keep their versions aligned.

## Make the smallest complete change

- Reuse the existing connection abstraction.
- Put a Direct-only mutation behind an explicit Direct check.
- Put Agent/MCP behavior in shared `ToolExecutor`, not two implementations.
- Do not hold `parking_lot` guards across `await`.
- Keep serialized Rust and TypeScript field names aligned.
- Preserve trust-boundary validation and useful errors.
- Add one focused regression check for non-trivial behavior.
- Update public docs in the same change when defaults, limits, persistence,
  security, or UI behavior changes.

## Local gate

Run from repository root:

```sh
npm ci
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

What this covers:

| Command | Coverage |
| --- | --- |
| `npm run build` | TypeScript checking and production Vite build |
| `cargo fmt --check` | Rust formatting |
| `cargo clippy ... -D warnings` | All Rust targets with the local embedding feature |
| `cargo test` | Demo creation, frozen-root compatibility, Kit and SQL checks |

Use `npm run tauri dev` for UI and integration behavior.

## Manual test matrix

Exercise only the rows affected by the change, but state what was and was not
tested in the PR.

| Area | Minimum useful check |
| --- | --- |
| Welcome/Direct | Open demo, disconnect, reopen from Recent |
| Credentials | Valid and invalid username/password combination |
| Server | Health, list tables, schema, count, one SQL query |
| Deck/Stars | Counts, recipes, fit/pan/zoom, table navigation |
| Table | Schema, row projection, REINDEX table/all |
| SQL | Query, truncation, CSV, DDL/DML if changed |
| ANN | Install, re-embed, rebuild, search, reopen durability |
| Agent | Save/probe, one tool call, key persistence warning |
| MCP HTTP | Health, initialize, tools/list, one tools/call |
| MCP stdio | Environment open, JSON-line request, clean EOF |
| About | All documents and credit filters load |

Use synthetic data. Never commit production roots, credentials, model keys, or
private endpoint URLs.

## Frontend changes

- Keep keyboard navigation and visible labels intact.
- Preserve `aria-label`, button types, form labels, and disabled explanations.
- Use ASCII hyphens in UI copy, not em dashes.
- Test the minimum window size `1100 × 700`.
- Keep errors in the shared banner when possible.
- Avoid rendering full embeddings when a compact preview works.
- Run the production build, not only hot reload.

Screenshots in `docs/images/` should be updated when a documented surface
materially changes. Use a synthetic demo root. `VITE_AUTO_OPEN_DB` can make
repeatable screenshot startup easier:

```sh
VITE_AUTO_OPEN_DB=/absolute/path/to/demo npm run tauri dev
```

## Database and SQL changes

Consider both `Connection::Direct` and `Connection::Server`.

- Direct can read full core schema and foreign keys.
- Server uses Kit schema and currently lacks FK metadata.
- Direct ANN install/rebuild is allowed; Server UI install is not.
- Semantic search is data/schema read-only; Direct provider registration is
  process-local and must not rewrite embedding-source metadata.
- Server operations are constrained by daemon support and authorization.
- Returned SQL rows are capped after Arrow batches arrive.

Do not imply whole-database semantic search. The current operation is
table-scoped.

## Agent and MCP changes

Agent and MCP share `src-tauri/src/mcp/tools.rs`.

When adding or changing a tool:

1. update the JSON schema in `tool_definitions`,
2. implement it once in `ToolExecutor`,
3. keep Direct/Server behavior explicit,
4. classify whether it can write,
5. update [docs/mcp.md](docs/mcp.md),
6. update [docs/agent.md](docs/agent.md) if model behavior changes,
7. test HTTP and Agent conversion when relevant.

Do not add a mutating tool without documenting that it writes and how users
limit its authority.

## Dependency changes

### npm

Update the lockfile with npm, run the frontend build, then regenerate legal
inventories.

### Cargo

Keep these on one train:

```toml
mongreldb-core
mongreldb-query
mongreldb-client
mongreldb-kit
```

Run tests against the frozen compatibility fixture. Do not replace the fixture
merely because dependencies changed.

### Legal regeneration

Install `cargo-about` if needed:

```sh
cargo install cargo-about
```

Then:

```sh
npm ci
scripts/regen-credits.sh
```

Commit and review:

```text
src-tauri/legal/crates.json
src-tauri/legal/third-party.md
src-tauri/legal/npm-packages.json
src-tauri/legal/npm-third-party.md
src-tauri/legal/acknowledgments.md
```

## Compatibility fixture

The archive:

```text
src-tauri/tests/fixtures/sample-demo-v0.64.5.tar.gz
```

must remain old during routine upgrades. Its purpose is to prove the current
engine can open data written by an older train.

Regenerate only for a deliberate golden-schema/version transition, after
reviewing [the fixture policy](src-tauri/tests/fixtures/README.md):

```sh
scripts/gen-compat-fixture.sh
```

The ignored Rust test also requires `REGEN_COMPAT_FIXTURE=1`.

## Documentation changes

Public docs are part of the feature.

- Keep commands runnable from repository root unless the text says otherwise.
- State Direct/Server differences.
- State defaults, upper bounds, persistence, and destructive behavior.
- Cross-link one canonical detailed guide instead of copying long sections.
- Use relative links for repository files.
- Do not claim a remote embedding UI, read-only Agent, authenticated MCP, or
  full SSE transport unless the implementation changes.
- Run the local-link check described in the PR or manually inspect every
  changed relative link.

## Version and release consistency

Application version appears in:

```text
package.json
package-lock.json
src-tauri/Cargo.toml
src-tauri/tauri.conf.json
```

Keep all four synchronized. An engine-train bump also updates:

```text
src-tauri/Cargo.toml
src-tauri/Cargo.lock
README.md when it names the train
src-tauri/legal/
acknowledgment/version references
```

Before tagging a release:

1. run the full gate,
2. complete affected manual checks,
3. confirm app and engine versions in About,
4. review bundled licenses,
5. build native packages on intended hosts,
6. test one packaged binary, not only `tauri dev`,
7. review README release/install links.

Signing and publication credentials are outside the repository.

## Pull request checklist

Include:

- **What** changed,
- **Why** it is needed,
- **How to test** with exact commands,
- **Direct/Server impact**,
- **Security/data impact**,
- **Screenshots** for visible changes,
- **Risk and untested paths**.

Reviewers look for:

- one coherent change,
- root-cause fixes,
- minimal new dependencies,
- accurate tool/write boundaries,
- a regression check where behavior can break,
- current docs and legal inventories,
- no unrelated generated or local files.

Never add AI contributors, co-author trailers, generated-by footers, bot
attribution, or similar metadata.

## Licensing

Contributions are accepted under **MIT OR Apache-2.0**. By submitting a pull
request, you agree that your contribution may be distributed under either
license.

## Questions

- Viewer behavior or UX: open a GitHub issue with synthetic reproduction data.
- MongrelDB engine behavior: consult
  [visorcraft/MongrelDB](https://github.com/visorcraft/MongrelDB).
- Vulnerabilities: use the private process in [SECURITY.md](SECURITY.md).
