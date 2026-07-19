# Security

This document describes how to report vulnerabilities in MongrelDB Viewer and
outlines security-relevant properties of the application.

## Reporting a vulnerability

**Do not file a public GitHub issue, discussion, or pull request for security
problems.** Report privately through **GitHub's private vulnerability reporting**:

1. Open the repository **Security** tab.
2. Click **Report a vulnerability**.
3. Fill in the advisory form with as much detail as you can.

Please include:

- a description of the issue and its impact,
- step-by-step reproduction steps,
- MongrelDB Viewer version (or git SHA), OS, and Rust/Node versions when relevant,
- configuration, logs, or a proof-of-concept,
- a suggested fix or mitigation, if you have one.

### What to expect

- **Acknowledgement** of your report within a few days.
- An initial assessment and, where confirmed, a remediation plan.
- Progress updates through the private advisory thread until the issue is
  resolved.
- Credit for responsible disclosure in the advisory, unless you prefer to remain
  anonymous.

Please give us a reasonable opportunity to ship a fix before any public
disclosure.

## Scope

In scope:

- The MongrelDB Viewer desktop application (Tauri + React + Rust).
- Local MCP HTTP bridge started from the app.
- Embedding provider configuration handling (API keys, remote endpoints).
- How the viewer opens, queries, and disconnects from MongrelDB roots or
  `mongreldb-server`.

Related but separate:

- Vulnerabilities in the **MongrelDB engine** itself should be reported to
  [`visorcraft/MongrelDB`](https://github.com/visorcraft/MongrelDB) under that
  project's security policy.
- System WebView / WebKitGTK / OS packages are maintained by platform vendors.

## Application security notes

### Connection modes

- **Direct** open embeds `mongreldb-core` in-process and takes the exclusive
  database lock. Only one exclusive client should open a root at a time.
- **Server** mode talks to `mongreldb-server` over HTTP. Auth depends on how the
  server is configured (bearer token / basic auth as supported by the client).
  Treat network exposure of the server as an operator responsibility.

### Local MCP bridge

The in-app MCP HTTP endpoint is intended for **local agent tooling**. Do not
expose it on untrusted networks without additional access control. Prefer binding
to loopback when possible.

### Credentials and secrets

- Catalog credentials, encryption passphrases, chat API keys, and remote
  embedding credentials are provided by the user and held in process memory for
  the session. Do not log secrets.
- Prefer OS secret storage for long-lived keys when integrating new features;
  do not write API keys into the repository or demo fixtures.

### Embeddings

- Local MiniLM models may be downloaded on demand by `fastembed`. Treat model
  cache directories as untrusted for multi-user machines.
- Remote embedding endpoints are user-configured; TLS and endpoint trust are
  operator choices.

### WebView surface

The UI runs inside a WebView (WebKitGTK / WebView2 / WKWebView). Content Security
Policy is configured in `src-tauri/tauri.conf.json`. Changes that widen `connect-src`,
`img-src`, or script sources should be called out in the PR risk section.

## Dependency security

Viewer dependencies include Tauri, React, Tokio, Reqwest, Arrow, and the
MongrelDB crates from crates.io. Prefer:

- Dependabot / advisory alerts on this repository,
- regenerating `src-tauri/legal/` after dependency bumps,
- minimal new dependencies with clear licenses.

Report dependency CVEs that are exploitable through Viewer behavior via the
private reporting flow above.

## Supported versions

Security fixes are applied to the latest release on `main` and published as a
patch release when appropriate. Older tags may not receive backports unless a
fix is still widely used.
