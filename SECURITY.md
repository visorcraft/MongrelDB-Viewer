# Security policy

This policy explains private vulnerability reporting, supported versions,
application trust boundaries, credential handling, and safe deployment.

## Report a vulnerability privately

Do not open a public issue, discussion, pull request, or test fixture for a
suspected vulnerability.

Use
[GitHub private vulnerability reporting](https://github.com/visorcraft/MongrelDB-Viewer/security/advisories/new):

1. Open the repository's **Security** tab.
2. Select **Report a vulnerability**.
3. Submit a private advisory draft.

Include:

- affected Viewer version or commit,
- OS and Direct/Server mode,
- clear impact and attacker requirements,
- minimal reproduction steps or proof of concept,
- relevant configuration and exact errors,
- whether the demo database reproduces it,
- suggested mitigation when known.

Remove API keys, passphrases, bearer tokens, passwords, and unrelated database
rows. If sample data is required, create a minimal synthetic root.

Maintainers aim to acknowledge reports within a few days, assess severity,
coordinate a fix and release, and keep discussion in the private advisory.
Please allow reasonable remediation time before public disclosure. Credit is
welcome and can be omitted on request.

## Supported versions

Security fixes target the latest release and `main`. Older tags may not receive
backports. Verify the current release on
[GitHub Releases](https://github.com/visorcraft/MongrelDB-Viewer/releases).

## Scope

In scope:

- Tauri desktop application,
- React/WebView frontend and Tauri command boundary,
- Direct and Server connection handling,
- SQL and schema inspection behavior added by Viewer,
- local embedding integration,
- Agent configuration and tool loop,
- in-app MCP HTTP server and stdio transport,
- local storage of Viewer configuration and in-memory handling of secrets,
- packaged legal/runtime assets when they create an exploitable condition.

Report separately:

- MongrelDB engine/query/client vulnerabilities to
  [visorcraft/MongrelDB](https://github.com/visorcraft/MongrelDB) under its
  policy,
- WebKitGTK, WebView2, WKWebView, GTK, and OS runtime vulnerabilities to their
  vendors,
- upstream model or package vulnerabilities to their projects, unless Viewer
  exposes a specific exploitable path.

## Security model

MongrelDB Viewer is a developer tool running with the current OS user's
permissions. It is not a sandbox, database proxy, authorization gateway, or
read-only client.

| Surface | Security property |
| --- | --- |
| Direct | Read-write embedded engine with an exclusive root lock |
| Server | Read-write capabilities allowed by daemon and supplied identity |
| SQL | Arbitrary engine SQL, including DDL/DML/maintenance |
| Agent | Remote model can choose any shared tool without per-call confirmation |
| In-app MCP | Unauthenticated loopback JSON-RPC with read and write tools |
| Stdio MCP | Parent process controls tool requests and receives results |
| WebView storage | Persistent recents and non-secret Agent settings |

Use disposable data while learning. Use least-privilege Server credentials when
available.

## Credential handling

### Connection secrets

Direct fields:

- catalog username,
- catalog password,
- encryption passphrase.

Server fields:

- bearer token,
- basic-auth username,
- basic-auth password.

These values remain in process/UI state for the running application. Recent
entries store only mode, path/URL, label, and timestamp. They do not store
connection passwords, tokens, or passphrases.

The application does not log these values intentionally. Do not include them in
screenshots or public issue output.

### Agent API key

The Agent API key remains in UI memory for the current process. It is sent only
to the configured model endpoint. Clicking **Save** writes:

```text
mongreldb-viewer.chat-config
```

to WebView `localStorage`. The stored JSON includes:

- Base URL,
- model.

The API key is deliberately excluded. On first launch after upgrading from an
older build, Viewer rewrites that WebView entry with only the allowed fields,
removing any legacy `apiKey` or other values.
Use scoped, revocable keys and re-enter the key after each process start.

### Stdio environment

Server credentials can be passed with:

```text
MONGRELDB_VIEWER_TOKEN
MONGRELDB_VIEWER_USER
MONGRELDB_VIEWER_PASSWORD
```

Environment secrets are visible to the child process and may be observable
through OS process inspection or client configuration. Inject them with the
parent client's secret facility and do not commit them.

## Direct mode

Direct embeds `mongreldb-core` and owns the database root's exclusive lock.
The lock coordinates exclusive openers; it does not make unsafe SQL read-only
or protect files from the OS user.

Safe handling:

- disconnect before another exclusive opener,
- disconnect before backup, move, or upgrade operations,
- never delete lock, WAL, catalog, or table files to force an open,
- test cross-version opens on a copy,
- keep root permissions limited to intended OS users.

Viewer's demo creation refuses non-empty directories and existing root markers.
That guard protects unrelated files, but it is not a substitute for reviewing
the selected path.

## Server mode

Viewer supports bearer and basic authentication through the official client.
Transport security comes from the configured URL.

- Use `https://` across untrusted networks.
- Plain HTTP exposes auth headers, SQL, and returned rows to network observers.
- Viewer does not provide a VPN, tunnel, certificate pinning, or daemon access
  policy.
- The server decides whether DDL, DML, REINDEX, and ANN SQL are allowed.
- Use a database identity with only required permissions.

## SQL

The workbench and `execute_sql` tool submit arbitrary SQL. Statement
classification is display metadata only.

There is:

- no `SELECT` allowlist,
- no write confirmation,
- no query sandbox,
- no automatic rollback around model-generated SQL,
- no query cancellation control in the current UI.

Review SQL before running it manually. For Agent/MCP, constrain risk with a
disposable root or server-side least privilege.

## Agent

Agent sends data to the configured model provider. Requests can include:

- prompts and full current conversation,
- tool definitions,
- table names and schemas,
- SQL statements and returned rows,
- ANN hits and semantic provenance,
- tool errors.

The model can call `execute_sql`, `install_dense_ann`, and `reindex` without a
separate approval dialog. The built-in instruction prefers inspection and
`SELECT`, but prompt text is not an authorization control.

The conversation remains in process memory only while its database connection
stays active. Disconnect and every successful new connection clear messages,
tool results, and draft input before another database can use them.

Use HTTPS for remote model endpoints. Viewer sends bearer authentication and
does not implement provider-specific security headers.

## MCP

### In-app HTTP

The public UI binds MCP to `127.0.0.1` and a user-selected port. It has:

- no authentication,
- no client allowlist,
- no tool-level authorization,
- mutating tools,
- no automatic stop on database disconnect.

Any process running as a local user that can reach the port can invoke tools.
Keep the OS account trusted, stop MCP when unused, and do not forward the port
to another host.

The backend has an internal host argument, but the desktop UI intentionally
does not expose non-loopback binding.

### Stdio

The parent process can invoke every tool. A Direct stdio process owns a
separate exclusive database connection and cannot share GUI Direct state.
Server stdio uses its own authenticated client.

Review the MCP client's trust and extension ecosystem before granting it
database access.

## Embeddings and model cache

The default MiniLM model may download on first use and is cached under the OS
cache directory. On a shared machine:

- restrict cache directory permissions,
- treat model files as untrusted inputs if another user can replace them,
- verify network/proxy trust for initial download,
- avoid sending sensitive source text to a remote embedding provider added by
  a custom build or internal command.

The current desktop UI uses local MiniLM for ANN and has no
remote-embedding configuration form.

## WebView and Tauri boundary

The frontend runs in WebKitGTK, WebView2, or WKWebView. Tauri capabilities are
limited to required core window/event APIs, opener, and file dialogs.

The configured CSP is:

```text
default-src 'self';
connect-src 'self' http://127.0.0.1:* http://localhost:* https:;
img-src 'self' data: asset: https:;
style-src 'self' 'unsafe-inline';
font-src 'self' data:
```

Changes that widen script sources, Tauri permissions, opener behavior, or
network destinations need explicit security review. Rust-side Reqwest calls
are outside the WebView's `connect-src` enforcement.

## Local files and privacy

Viewer stores:

- Recent paths/URLs in WebView storage,
- saved Agent URL/model configuration in WebView storage,
- the current Agent API key only in process memory,
- demo-used flag in OS config storage,
- MiniLM files in OS cache storage,
- Linux desktop/icon files under `~/.local/share`.

Viewer does not implement telemetry or an automatic update service in this
repository.

See [operations](docs/operations.md) for exact keys, typical paths, network
triggers, and retained data.

## Dependency security

The application includes Rust, npm, and platform runtime dependencies. For
dependency changes:

1. review advisories and release notes,
2. keep the four `mongreldb-*` crates on one aligned train,
3. run the full local gate,
4. regenerate `src-tauri/legal/`,
5. review CSP and permissions,
6. test Direct, Server, Agent, and MCP paths affected by the update.

A dependency CVE is in Viewer scope when an attacker can reach it through
Viewer behavior or packaging.

## Hardening checklist

- Run Viewer as a normal, dedicated OS user when handling sensitive roots.
- Keep Direct roots and model cache private to that user.
- Use tested backups and disconnect before backup.
- Prefer Server mode with least privilege for model-driven exploration.
- Use HTTPS for remote database and model endpoints.
- Use scoped Agent keys and close Viewer to clear them from process memory.
- Keep MCP loopback-only and stop it after use.
- Inspect model-generated tool messages for writes.
- Update Viewer, OS WebView, and system libraries.
- Never post real roots or secrets to public issues.
