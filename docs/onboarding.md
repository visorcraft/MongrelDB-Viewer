# Onboarding

This guide walks through first launch: connecting to a database, creating a demo
root, and navigating the rail.

## Install and run

```bash
git clone https://github.com/visorcraft/MongrelDB-Viewer.git
cd MongrelDB-Viewer
npm install
npm run tauri dev
```

Requirements: Rust **1.88+**, Node.js **22+**, and platform Tauri libraries
([prerequisites](https://v2.tauri.app/start/prerequisites/)).

On Linux, the app sets WebKitGTK Wayland-safe defaults at startup. You should
not need manual `GDK_BACKEND=x11` workarounds for normal use.

## Welcome screen

When no database is open, the Welcome screen offers two connection modes.

### Direct folder (exclusive)

1. Select **Direct folder**.
2. Enter a path to a MongrelDB root, or use **Browse** to pick a directory.
3. Optionally set catalog **username/password** and/or an encryption
   **passphrase** if the root requires them.
4. Click **Open database**.

A valid root typically contains engine artifacts such as `CATALOG`, `_meta`,
and/or `tables`. Direct mode embeds the engine in-process and holds the
exclusive lock - only one exclusive client should open a root at a time.

**Create demo DB** seeds a multi-table sample (tenants, authors, documents,
events, tags, …) into an **empty** directory. It refuses to overwrite an
existing MongrelDB root. Demo creation may download a local embedding model
when ANN is included. The root includes engine files (`CATALOG`, `_meta`,
`tables`) plus a Kit sidecar `kit_schema.json` so Kit-backed clients such as
**Mongrel** can open the same directory.

### mongreldb-server (multi-client)

1. Start a server separately, for example:

   ```bash
   mongreldb-server /path/to/db 8453
   ```

2. In the app, select **mongreldb-server**.
3. Enter the server URL (loopback example: `http://127.0.0.1:8453`).
4. Add bearer token or basic auth only if your server is configured for them.
5. Click **Connect to server**.

## After connect

The top bar shows:

- **Direct** or **Server** mode badge  
- Display path or URL  
- Engine version  
- **Path chip** in the top bar → confirm **Disconnect** (releases exclusive lock / drops server session)

The left **rail** switches pages:

| Rail | Page |
| ---- | ---- |
| Deck | Overview |
| Stars | Schema map |
| Table | Table browser |
| SQL | SQL workbench |
| ANN | Vector search |
| Agent | Chat |
| MCP | MCP bridge |
| Sync | Refresh overview |
| About | Product / licenses / credits |

## Command palette

Press the shortcut shown in the top bar (typically **Ctrl+F** / **⌘F** depending
on platform bindings in this build) when connected, or use the button next to
the connection pills. The palette jumps to pages, tables, and suggested queries.

## Keyboard

| Shortcut | Action |
| -------- | ------ |
| `1`-`7` | Switch views when not typing in an input |
| Ctrl/⌘+Enter | Run SQL (SQL page) |
| `?` | Shortcuts help |

## Safety tips

- Do not paste real production secrets into chat system prompts or screenshots.
- Prefer disconnecting via the path chip before closing the app when using Direct mode so the
  lock is released cleanly.
- Treat the MCP HTTP endpoint as local tooling - do not expose it on untrusted
  networks without extra controls.

Next: [Deck](deck.md) · [Create ANN search](ann.md) · [Full index](README.md)
