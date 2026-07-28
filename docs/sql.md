# SQL workbench

**SQL** sends MongrelDB/DataFusion SQL to the active Direct session or Server
endpoint and renders returned Arrow batches.

![SQL workbench](images/04-sql.png)

## Execute a request

1. Enter SQL.
2. Review it for writes or expensive scans.
3. Click **Run** or press Ctrl/Command+Enter.
4. Inspect statement kind, returned row count, truncation marker, and elapsed
   time.

Viewer submits the editor text as one engine request. Whether a particular
dialect feature or multi-statement string is accepted is determined by the
linked MongrelDB query layer or remote server.

The workbench has no query-cancel button. Use selective filters and limits for
large data.

## Read-write behavior

The editor is not read-only. It can submit:

- `SELECT`, `WITH`, catalog queries, and joins,
- `INSERT`, `UPDATE`, and `DELETE`,
- supported DDL such as `CREATE`, `ALTER`, and `DROP`,
- maintenance statements such as `REINDEX`.

There is no separate confirmation for DDL or DML. Server authorization decides
what remote SQL can change. Back up important roots and use least-privilege
server credentials.

## Returned row cap

The normal workbench calls the backend with its default cap:

```text
500 rows
```

The backend accepts caps from 1 through 10,000. Table Browser passes its
selected 25/50/100/250/500 limit. MCP `execute_sql` exposes `max_rows`.

Truncation happens after the query layer returns Arrow batches. It limits data
kept and rendered by Viewer, but it is not a substitute for SQL `LIMIT` and may
not reduce engine work.

Result metadata:

| Field | Meaning |
| --- | --- |
| `rowCount` | Rows retained after the cap |
| `truncated` | More rows were returned than retained |
| `elapsedMs` | Viewer-side execution/conversion duration |
| `statementKind` | Display classification from the first SQL token |

The UI displays `500+ rows` when the cap truncates a result.

## Statement classification

Viewer labels common first tokens:

| Label | Tokens |
| --- | --- |
| `query` | `SELECT`, `WITH`, `EXPLAIN`, `SHOW`, `DESCRIBE`, `DESC`, `VALUES` |
| `dml` | `INSERT`, `UPDATE`, `DELETE`, `MERGE`, `TRUNCATE` |
| `ddl` | `CREATE`, `ALTER`, `DROP`, `RENAME` |
| `maintenance` | `REINDEX`, `ANALYZE`, `VACUUM`, `OPTIMIZE` in Direct mode |
| `txn` | `BEGIN`, `COMMIT`, `ROLLBACK`, `SAVEPOINT` in Direct mode |
| `session` | `ATTACH`, `DETACH`, `PRAGMA`, `SET`, `USE` in Direct mode |

This is a UI label only. It does not prove support, safety, or transaction
semantics.

Server classification currently has a smaller recognized set, so some valid
server statements can display their lowercase first token instead.

## Results and conversion

Viewer converts Arrow values to JSON-compatible cells:

- booleans and safe-range numeric scalars remain scalar,
- `Int64`/`UInt64` outside JavaScript's exact integer range
  `-9,007,199,254,740,991..9,007,199,254,740,991` become decimal strings,
- UTF-8 text and UTF-8 Bytes become strings,
- non-UTF-8 Bytes become a `\x` hexadecimal preview,
- null remains null,
- long f32 fixed-size lists become `{ "dim": ..., "preview": [...] }`,
- unsupported Arrow types show a type placeholder.

For long embeddings, the preview contains the first four and last four values.
Non-UTF-8 byte previews contain at most 64 bytes.

**Copy CSV** copies only the currently retained rows. It:

- writes the displayed column header,
- JSON-encodes object cells,
- quotes fields containing comma, quote, or newline,
- doubles quotes inside quoted fields.

Clipboard failure is not currently surfaced as a dedicated error.

## Editor helpers

- **Sample** inserts `SELECT * FROM <first-table> LIMIT 25`, or `SELECT 1`.
- **Copy** copies current SQL.
- A result remains visible until replaced or the database disconnects.
- SQL run from Table's **Sample rows** switches here and executes immediately.

## History

Successful workbench requests are deduplicated and kept in process memory:

```text
maximum retained: 12
maximum shown as chips: 6
```

Clicking a history chip loads SQL without running it. History is not persisted
across application exit. It can contain sensitive literals for the rest of the
process lifetime.

## Recipes

Viewer creates up to 28 unique recipes from loaded schema. It prioritizes:

- table browse and count,
- Bitmap/categorical group counts,
- numeric sort and score thresholds,
- recent rows by timestamp-like columns,
- text substring filters,
- equality templates,
- non-null filters,
- vector-ready hints.

Recipe filters show available categories. Clicking a recipe loads it into the
editor without running it. Deck cards and command-palette recipe actions can
run a selected query immediately.

Generated identifiers come from the database schema. Review recipes before
running them against unusual quoted identifiers or important data.

## Demo queries

List tables:

```sql
SELECT name
FROM information_schema.tables
ORDER BY name;
```

Count document states:

```sql
SELECT status, count(*) AS n
FROM documents
GROUP BY status
ORDER BY n DESC;
```

Join tenants, authors, and documents:

```sql
SELECT
  d.id,
  cast(t.name AS varchar) AS tenant,
  cast(a.name AS varchar) AS author,
  cast(d.body AS varchar) AS body,
  d.score
FROM documents d
JOIN tenants t ON d.tenant_id = t.id
JOIN authors a ON d.author_id = a.id
ORDER BY d.score DESC;
```

Use the FM-friendly text shape:

```sql
SELECT id, body
FROM documents
WHERE cast(body AS varchar) LIKE '%retrieval%'
LIMIT 20;
```

Inspect recent events:

```sql
SELECT id, document_id, kind, payload, ts
FROM events
ORDER BY ts DESC
LIMIT 10;
```

Update the disposable demo:

```sql
UPDATE documents
SET status = 'active'
WHERE id = 3;
```

Run table maintenance:

```sql
REINDEX documents;
```

Run database-wide maintenance:

```sql
REINDEX;
```

## ANN SQL

The ANN guide documents `CREATE INDEX` combinations and the engine functions
used by Viewer:

```text
ann_search
ann_search_exact
retrieve_text
```

Use the ANN page when the query vector should be created from text. The SQL
workbench does not translate natural-language text into a vector by itself.

## Direct and Server differences

| Detail | Direct | Server |
| --- | --- | --- |
| Executor | Local `MongrelSession` | `MongrelClient::sql` |
| Timeout | Engine/session behavior | Client request timeout 120 seconds |
| Statement support | Linked query crate | Remote daemon |
| Authorization | Filesystem/catalog ownership | Server auth and policy |
| Result conversion | Shared Arrow-to-JSON path | Shared Arrow-to-JSON path |

If a query works in Direct but not Server, compare engine trains, daemon
capabilities, authorization, and schema sidecar exposure.

Related: [Table](table.md) · [ANN](ann.md) · [Connections](connections.md) ·
[Troubleshooting](operations.md#troubleshooting)
