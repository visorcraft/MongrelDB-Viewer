# Configuration, local data, and troubleshooting

MongrelDB Viewer has intentionally little configuration. Most values live in
the connection form, Agent form, or MCP page. This guide records every
repository-defined environment variable, persistent local value, network
boundary, and common failure path.

## State and persistence

| Data | Storage | Lifetime | Contains secrets |
| --- | --- | --- | :---: |
| Active Direct/Server connection | Rust process memory | Until disconnect or exit | May hold credentials |
| Connection form fields | React memory | Until exit | Yes |
| Recent paths and URLs | WebView `localStorage` | Across launches | No passwords or tokens |
| Agent base URL and model | WebView `localStorage` after **Save** | Across launches | No |
| Agent API key | React memory | Until process exit | **Yes** |
| Agent conversation and tool results | React memory | Until disconnect, another successful connection, or exit | Can contain database data |
| SQL history | React memory | Until process exit | Can contain SQL literals |
| Demo-used flag | OS config file plus legacy WebView flag | Across launches | No |
| MiniLM files | OS cache directory | Until manually removed | No |
| ANN schema and vectors | The opened MongrelDB root | Durable | Application data |
| Linux desktop/icon integration | `~/.local/share` | Across launches | No |

Saving Agent configuration writes this WebView key:

```text
mongreldb-viewer.chat-config
```

Its JSON contains only `baseUrl` and `model`. The API key stays in React memory
and must be entered again after process restart. On upgrade, loading an older
entry rewrites it with that allowlist, removing its legacy `apiKey` and any
other fields.

Other WebView keys:

```text
mongreldb-viewer.recents.v1
mongreldb-viewer.demo-used
```

The durable demo flag is stored at the OS config directory:

| Platform | Typical path |
| --- | --- |
| Linux | `$XDG_CONFIG_HOME/mongreldb-viewer/flags.json` or `~/.config/mongreldb-viewer/flags.json` |
| macOS | `~/Library/Application Support/mongreldb-viewer/flags.json` |
| Windows | `%APPDATA%\\mongreldb-viewer\\flags.json` |

The WebView storage directory is controlled by Tauri and the platform WebView.
Its physical location is platform-dependent.

The demo button hides after `demoUsed` becomes true. The older WebView flag is
migrated into `flags.json`, so changing only one copy may not reset the button.
There is no in-app reset control.

## Model cache

`all-MiniLM-L6-v2` is loaded on first ANN install, re-embed, or semantic query.
The cache follows the OS cache directory:

| Platform | Typical path |
| --- | --- |
| Linux | `$XDG_CACHE_HOME/mongreldb-viewer/models` or `~/.cache/mongreldb-viewer/models` |
| macOS | `~/Library/Caches/mongreldb-viewer/models` |
| Windows | `%LOCALAPPDATA%\\mongreldb-viewer\\models` |

Viewer creates the directory when possible. The model loader owns download and
file layout below it.

## Environment variables

### Headless MCP

These are read only when the binary starts with `--mcp-stdio`:

| Variable | Meaning |
| --- | --- |
| `MONGRELDB_VIEWER_PATH` | Direct root opened by the stdio process |
| `MONGRELDB_VIEWER_SERVER` | Server URL opened by the stdio process |
| `MONGRELDB_VIEWER_TOKEN` | Server bearer token |
| `MONGRELDB_VIEWER_USER` | Server basic-auth username |
| `MONGRELDB_VIEWER_PASSWORD` | Server basic-auth password |

If both `MONGRELDB_VIEWER_PATH` and `MONGRELDB_VIEWER_SERVER` are set, Direct
path wins. The Direct stdio path has no environment variables for catalog
username/password or encryption passphrase. Server basic auth requires both
`MONGRELDB_VIEWER_USER` and `MONGRELDB_VIEWER_PASSWORD`.

If neither connection variable is set, the MCP process still starts, but
database tools return `no database is open`.

### Development

| Variable | Meaning |
| --- | --- |
| `VITE_AUTO_OPEN_DB` | Auto-open a Direct path once at frontend startup |
| `TAURI_DEV_HOST` | Bind Vite to a development host and configure HMR on port `1421` |

`VITE_AUTO_OPEN_DB` is compiled into the frontend environment and is intended
for development, screenshots, and controlled test setups. Do not place
credentials in a `VITE_*` variable.

### Linux display

| Variable | Default set by Viewer | Purpose |
| --- | --- | --- |
| `WEBKIT_DISABLE_DMABUF_RENDERER` | `1` | Avoid Wayland DMA-BUF renderer failures |
| `WEBKIT_DISABLE_COMPOSITING_MODE` | `1` | Avoid hybrid-GPU compositing failures |

Viewer sets these only when they are absent. Export a different value before
launch to override it.

## Ports and endpoints

| Service | Default | Notes |
| --- | --- | --- |
| Vite development server | `http://localhost:1420` | Fixed, strict port |
| Vite HMR with `TAURI_DEV_HOST` | port `1421` | Development only |
| Example MongrelDB server | `http://127.0.0.1:8453` | User-configured |
| In-app MCP | `http://127.0.0.1:7337/mcp` | Port editable, host fixed to loopback by UI |
| Ollama example | `http://127.0.0.1:11434/v1` | User-configured Agent endpoint |

The Tauri content security policy permits application resources, HTTPS
connections, and HTTP loopback connections. Agent HTTP is performed by the
Rust backend. The MCP listener has no authentication.

## Network activity

Viewer has no telemetry or automatic update service in this repository.
Network activity occurs when a user asks for it:

| Trigger | Destination | Data sent |
| --- | --- | --- |
| Connect in Server mode | Configured `mongreldb-server` | Auth headers, schema/count requests, SQL, query vectors |
| First local-model load | Model source used by `fastembed` | Model download request |
| Save Agent configuration | Configured base URL | `GET .../models` probe with bearer key |
| Send Agent message | Configured base URL | Transcript, tool definitions, later tool results |
| External MCP client | Loopback MCP listener | JSON-RPC requests and tool results |

Agent tool results can contain arbitrary rows from the open database. The
remote provider receives them as the model loop continues.

## Safe operating checklist

Before opening important Direct data:

1. Confirm no other exclusive client owns the root.
2. Work from a tested backup when trying DDL, ANN rebuild, or model-driven
   tools.
3. Treat SQL, Agent, and MCP as read-write surfaces.
4. Keep MCP on loopback and stop it when unused.
5. Use HTTPS for a non-loopback database or chat endpoint.
6. Disconnect before backups, upgrades, or moving a root.

After external DDL or server-side changes, click **Sync**. After SQL from the
workbench, Viewer refreshes overview and insights automatically.

## Troubleshooting

### Application does not build

Run:

```sh
rustc --version
node --version
npm ci
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Required minimums are Rust 1.88 and Node.js 22. On Linux, a linker or
`webkit2gtk` error usually means Tauri system packages are missing. Follow the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

If Vite reports port `1420` is in use, stop the existing process. The
configuration intentionally uses `strictPort: true`.

### Linux window crashes with Wayland protocol error 71

Viewer already defaults:

```sh
WEBKIT_DISABLE_DMABUF_RENDERER=1
WEBKIT_DISABLE_COMPOSITING_MODE=1
```

Check that a launcher or wrapper did not override them. As a diagnostic only,
compare an X11 session:

```sh
GDK_BACKEND=x11 npm run tauri dev
```

Keep native Wayland when the default variables solve the issue.

### Linux taskbar icon is generic

Run:

```sh
scripts/install-icons-linux.sh
```

Then restart the application and, if needed, the desktop shell. GUI startup
also attempts this installation automatically.

### Direct root will not open

Common causes:

- another exclusive process owns the root,
- the selected directory is not a MongrelDB root,
- username/password are incomplete,
- passphrase or catalog credentials are wrong,
- filesystem permissions block the root,
- the root was written by an incompatible engine train.

For `invalid credential combination`, provide username and password together.
For a lock error, disconnect or stop the other exclusive process. Do not delete
lock or WAL files by hand.

### Demo creation is refused

Expected errors include:

```text
Demo path must be empty or missing (won’t touch existing files)
Refusing to create a demo database: ... already looks like a MongrelDB root
```

Choose a new empty directory. Viewer deliberately does not clean or overwrite
the selected path.

### Server cannot connect

`cannot reach mongreldb-server at ...` means the health request failed. Check:

- URL, scheme, host, and port,
- daemon process,
- firewall or proxy,
- TLS trust,
- 5-second connect timeout.

`connected but list_tables failed` means health answered but table listing
failed, commonly from authorization or server-root configuration.

### Overview or Sync is slow in Server mode

Viewer requests schema and exact count for every table. Use a lower-latency
server path, reduce catalog size, or inspect server count performance. Sync is
manual, so avoid repeated clicks while a prior request is busy.

### SQL returns too few rows

The workbench caps returned rows at 500. MCP `execute_sql` accepts
`max_rows` from 1 through 10,000. A `truncated` result means more rows were
returned by the engine than Viewer kept.

Add filters and `LIMIT` in SQL. Copy CSV exports only the rows currently held
by Viewer.

### Local embedding model fails

Errors start with:

```text
embedding error: failed to load all-MiniLM-L6-v2
```

Check:

- network access on first load,
- write permission and free space in the model cache,
- proxy or TLS configuration used by the model downloader,
- whether the app was built with default `local-embeddings`.

An offline demo may have zero vectors. After model access works, select
`documents.body` and use **Re-embed from text column**.

### ANN install is disabled

The UI requires:

- Direct mode,
- a selected table,
- loaded schema,
- at least one text-like Bytes, String, UTF, JSON, Text, or Char column.

Server mode cannot use the button. A table with existing ANN shows rebuild and
re-embed actions instead.

### ANN reports a type or dimension conflict

New installs use column `embedding` with dimension 384. If that name already
exists with another type or dimension, install fails instead of changing it.
Existing ANN actions target the first ANN index's actual column and dimension.

Product quantization requires a positive `num_subvectors` that divides the
target dimension. The UI default of 48 fits 384. Only 8 bits per subvector are
accepted.

### Semantic search returns weak or no results

- Confirm vectors were embedded, not zero-filled.
- Confirm the selected table has an ANN index and that its recorded provider is
  available.
- Use a text column that represents the desired meaning.
- Lower the default minimum cosine score `0.25` to admit weaker results.
- Increase `k`, up to the UI maximum of 100.
- Use Dense quantization when accurate cosine behavior matters.
- In Server mode, confirm the daemon supports the ANN SQL functions.

### Agent Save probe fails

Save probes a models endpoint derived from Base URL. The chat request uses a
Chat Completions URL derived separately.

For a conventional endpoint, use:

```text
https://host.example/v1
```

Viewer sends bearer authentication. Providers that require a different header,
query parameter, Responses API, or nonstandard tool-call format need a
compatible gateway.

API key is optional. Leave it empty for an unauthenticated local endpoint.

### Agent returns `unexpected chat response`

Viewer expects:

```text
choices[0].message
```

The endpoint must implement OpenAI-style Chat Completions and tool calls. Check
the selected model supports tools and that the base URL is not a Responses-only
endpoint.

### MCP will not start

An error like:

```text
mcp error: bind 127.0.0.1:7337 failed: Address already in use
```

means another process owns the port. Choose a different port in MCP, or stop the
other listener.

Test a running endpoint:

```sh
curl http://127.0.0.1:7337/health
```

Expected shape:

```json
{"ok":true,"service":"mongreldb-viewer-mcp"}
```

If tools return `no database is open`, connect a database in the same GUI
process. For stdio, set `MONGRELDB_VIEWER_PATH` or
`MONGRELDB_VIEWER_SERVER`.

### MCP client connects but protocol features are missing

Viewer implements tools for MCP protocol version `2024-11-05`. Resources and
prompts list as empty. The `/sse` endpoint is only a minimal endpoint probe, not
a full event stream. Use the POST `/mcp` transport or stdio as documented in
[MCP](mcp.md).

## Diagnostics for bug reports

Include:

- Viewer version from About,
- OS and display server,
- Direct or Server mode,
- linked engine/query versions shown in Deck,
- exact error text,
- minimal reproduction steps,
- whether a demo root reproduces it,
- `rustc --version` and `node --version` for source builds.

Do not attach database roots, API keys, passphrases, bearer tokens, or
production rows to a public issue. Use [private vulnerability reporting](../SECURITY.md)
for security-sensitive findings.

Viewer does not configure a persistent application log file. Source builds show
build and startup output in the launching terminal; errors intended for users
appear in the application banner.

Related: [Installation](installation.md) · [Connections](connections.md) ·
[Security](../SECURITY.md) · [Contributing](../CONTRIBUTING.md)
