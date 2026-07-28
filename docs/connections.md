# Direct and server connections

Every database-facing surface uses one active connection. Choose Direct for an
embedded local root. Choose Server when the root is already served or must be
shared by multiple clients.

## Mode comparison

| Behavior | Direct folder | `mongreldb-server` |
| --- | --- | --- |
| Transport | In-process engine | HTTP |
| Concurrency model | One exclusive opener | Multiple clients |
| Root path visible to Viewer | Yes | No |
| Catalog username/password | Yes | Sent as server basic auth |
| Encryption passphrase | Yes | No Viewer field for server passphrase |
| Bearer token | No | Yes |
| SQL | Local `MongrelSession` | Server SQL endpoint |
| Schema source | Live core catalog | Server Kit schema endpoint |
| Foreign keys in Stars | Yes | Currently unavailable |
| Native semantic identity and `retrieve_text` | Yes | SQL fallback |
| ANN install/backfill/rebuild button | Yes | Disabled |
| REINDEX | Direct engine call | SQL sent to server |

Both modes support Deck, Table, SQL, Agent, and in-app MCP. A server must expose
the schema, count, and SQL operations used by those surfaces.

## Direct folder

### What opens

Choose a directory containing a MongrelDB root. Common storage markers are:

```text
CATALOG
_meta/
tables/
_wal/
```

**Open database** opens an existing root. It does not create a general-purpose
empty database. **Create demo DB** is the UI's creation path and only writes to
an empty or missing directory.

### Locking

The embedded engine takes the root's exclusive lock. While Viewer holds it:

- another Direct Viewer process cannot open the same root,
- a separate `--mcp-stdio` process cannot open the same root,
- another exclusive MongrelDB client should not open it,
- filesystem copying is not a safe live-backup strategy.

Use the path chip in the top bar, confirm **Disconnect**, and wait for the
Welcome screen before another exclusive opener or backup starts. Normal process
exit drops the engine handle, but an orderly disconnect makes ownership clear.

### Credentials

Supported combinations are:

| Username | Password | Passphrase | Result |
| --- | --- | --- | --- |
| Empty | Empty | Empty | Normal root |
| Empty | Empty | Set | Encrypted root |
| Set | Set | Empty | Catalog credentials |
| Set | Set | Set | Encrypted root with catalog credentials |

A username without a password, or a password without a username, is rejected
as `invalid credential combination`.

Direct credentials remain in the running UI state so the form can reuse them.
They are not written into the Recent entry. Enter them in the current form
before clicking a protected Recent root.

### Direct-only metadata

Direct inspection reads:

- exact table counts from the embedded engine,
- all column flags, including encrypted and embedding flags,
- index-specific options,
- table foreign keys and referenced column names,
- live ANN semantic identity when bound,
- linked engine and query build information.

Direct semantic search first tries engine-native `retrieve_text`, including
provider/model provenance, then falls back to ANN SQL.

## Server

Start a daemon separately. A common local command is:

```sh
mongreldb-server /path/to/db 8453
```

Then:

1. Select **mongreldb-server**.
2. Enter a full URL such as `http://127.0.0.1:8453`.
3. Add a bearer token and/or basic-auth username and password when required.
4. Click **Connect to server**.

Trailing slashes are removed from the stored URL. The client uses a 5-second
connect timeout and a 120-second request timeout.

Before accepting a connection, Viewer:

1. calls the server health endpoint,
2. calls `list_tables`,
3. loads each table's Kit schema and count for the overview.

Errors therefore distinguish an unreachable daemon from a daemon that answered
but denied or failed table listing.

### Server limitations

- The current adapter does not receive foreign keys, so Server-mode Stars has
  no table-to-table FK edges.
- The ANN install and rebuild controls are Direct-only. Use approved
  server-side SQL or administration tools instead.
- Semantic search cannot use the Viewer's process-local provider registry on
  the server. It embeds the query locally and sends ANN SQL.
- Exact server search may return a narrower projection than Direct search.
- The engine/query version shown by the current overview comes from the
  MongrelDB crates linked into Viewer. It is a compatibility-train indicator,
  not authoritative daemon inventory.
- Server capabilities and authorization decide whether DDL, DML, REINDEX, and
  ANN SQL succeed.

Server overview and refresh perform schema and count requests per table. Large
catalogs or expensive remote counts can make connect and **Sync** slower.

### Transport security

Use HTTPS when the server crosses an untrusted network. A plain `http://` URL
sends SQL, returned rows, bearer tokens, and basic-auth credentials without TLS.
Viewer does not add a tunnel or certificate pinning.

## Recent connections

Viewer stores at most eight Recent entries in WebView `localStorage` and shows
the first five on Welcome.

Stored:

- mode,
- Direct path or Server URL,
- display label,
- last-used timestamp.

Not stored in Recent:

- catalog username,
- catalog password,
- encryption passphrase,
- bearer token,
- server basic-auth username,
- server basic-auth password.

There is no in-app Clear Recent control.

Clicking a Recent entry attempts to connect immediately. Viewer passes the
credentials currently present in the form, but never copies them into the
Recent record. Enter the required Direct passphrase/catalog pair or Server
token/basic-auth pair before clicking a protected entry.

## Disconnect, refresh, and shared tools

- **Sync** reloads overview and graph data. It does not change connection mode.
- Running SQL through the workbench refreshes overview and insights after the
  request.
- Disconnect drops the active database handle and returns to Welcome.
- Disconnect does not automatically stop an already-running in-app MCP HTTP
  server. Its database tools return `no database is open` until a new
  connection is established, or until MCP is stopped.
- Disconnect and every successful new connection clear Agent messages, tool
  results, and draft input.

## Which mode should I use?

Use Direct when:

- the root is local,
- no other client needs it,
- full schema/foreign-key inspection matters,
- Viewer should install or rebuild ANN,
- native `retrieve_text` provenance matters.

Use Server when:

- multiple clients need the database,
- the root is remote,
- daemon-side authentication or operations are required,
- Viewer should not own the filesystem lock.

Related: [First launch](onboarding.md) · [ANN](ann.md) · [MCP](mcp.md) ·
[Operations](operations.md)
