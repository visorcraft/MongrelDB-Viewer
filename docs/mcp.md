# MCP bridge

**MCP** exposes MongrelDB Viewer tools to external agents (Claude Desktop,
Cursor, custom clients) over a local HTTP JSON-RPC endpoint. In-app Agent chat
and MCP can run together on the same open database.

## Start / stop

1. Connect a database.  
2. Open **MCP** on the rail.  
3. Choose host/port if needed (default loopback port is suitable for local
   tooling).  
4. **Start MCP** - status shows the endpoint URL.  
5. **Stop MCP** when finished.  

Prefer binding to loopback. Do not expose the MCP port on untrusted networks
without additional access control.

## Client configuration

The page can show a config snippet for common clients. Point the client at the
displayed URL (often `http://127.0.0.1:<port>/mcp`).

## Stdio mode

For terminal / IDE stdio transport, launch the binary with environment variables
instead of the in-app HTTP server:

```bash
MONGRELDB_VIEWER_PATH=/path/to/db \
  mongreldb-viewer --mcp-stdio
```

Optional server mode:

```bash
MONGRELDB_VIEWER_SERVER=http://127.0.0.1:8453 \
  MONGRELDB_VIEWER_TOKEN=<token-if-needed> \
  mongreldb-viewer --mcp-stdio
```

Never put real tokens in shell history files or committed scripts - use env vars
or a secret manager.

## Tools

Typical tools include:

- `list_tables`  
- `describe_table`  
- `database_overview`  
- `constellation`  
- `execute_sql`  
- `semantic_search`  
- `install_dense_ann` (Direct only; `rebuild=true` drops and recreates ANN)  
- `reindex` (optional `table`; whole DB when omitted)  
- `list_embedding_providers`  

Exact names appear in the MCP status / snippet panel.

Related: [Agent](agent.md) · [Onboarding](onboarding.md) · [Security](../SECURITY.md)
