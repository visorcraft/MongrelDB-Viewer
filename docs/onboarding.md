# First launch and demo

This walkthrough creates a safe local database, tours every page, runs SQL, and
tests semantic search.

## Before launch

For a source checkout:

```sh
npm ci
npm run tauri dev
```

Requirements and package builds are covered in
[installation](installation.md).

## Welcome screen

Welcome offers:

- **Direct folder** for an existing local root with one exclusive opener,
- **mongreldb-server** for a multi-client HTTP daemon,
- up to five recently used paths or URLs,
- **Create demo DB** until the first successful demo creation.

Connection credentials are not copied into Recent entries. Read
[connection modes](connections.md) before opening protected or production data.

## Create the demo

1. Keep **Direct folder** selected.
2. Enter a missing directory or choose an existing empty directory with
   **Browse**.
3. Leave username, password, and passphrase empty.
4. Click **Create demo DB**.
5. Allow the first local embedding model download when network access is
   available.

Viewer refuses a non-empty path and any path that already looks like a
MongrelDB root. It does not delete or overwrite the directory.

After success, the demo opens immediately and the create button hides on future
launches. Reopen it through **Recent** or **Direct folder**.

## Demo contents

```text
tenants
  ├──< authors
  ├──< documents >── authors
  │      ├──< events
  │      └──< document_tags >── tags
  └──< events
```

| Table | Rows | Purpose | Secondary indexes |
| --- | ---: | --- | --- |
| `tenants` | 2 | Tenant name and plan | Bitmap on `plan` |
| `authors` | 4 | Tenant authors and roles | Bitmap on `tenant_id`, `role` |
| `documents` | 8 | Text, status, score, vectors | Bitmap, FM, LearnedRange, dense HNSW ANN |
| `events` | 16 | Document activity and JSON payload | Bitmap and LearnedRange |
| `tags` | 5 | Tag names | Bitmap |
| `document_tags` | 12 | Document/tag join table | Bitmap |

The UI-created demo has:

- 47 rows across six tables,
- seven foreign keys,
- Bitmap, LearnedRange/PGM, FM, and ANN examples,
- `documents.embedding` as nullable `Embedding(384)`,
- `docs_ann` using HNSW with Dense quantization,
- vectors from `documents.body` when MiniLM loads,
- `kit_schema.json` for Kit-backed clients.

Sparse and MinHash are recognized Viewer capabilities but are not installed in
this small demo.

If MiniLM cannot load during demo creation, the database still opens with zero
vectors. Once the model is available, use **ANN** ->
**Re-embed from text column** with `documents.body`.

## Tour the application

### 1. Deck

Confirm:

- six tables,
- 47 total rows,
- secondary-index total,
- one vector-ready table,
- table cards and SQL recipe chips.

Click `documents` to open Table, or a recipe to open and run SQL.

### 2. Stars

Open **Stars**. Drag empty space to pan, scroll to zoom, and click
**Fit all**. Pink dashed edges show foreign-key relationships. Click a table
node to open its inspector.

### 3. Table

Select `documents`. Inspect:

- `Embedding(384)` on `embedding`,
- embedding source metadata,
- Bitmap, FM, LearnedRange, and ANN index rows,
- ANN algorithm and quantization,
- live rows below the inspector.

Use **Hide embeddings** before loading wide row samples.

### 4. SQL

Start with:

```sql
SELECT name
FROM information_schema.tables
ORDER BY name;
```

Count by status:

```sql
SELECT status, count(*) AS n
FROM documents
GROUP BY status
ORDER BY n DESC;
```

Join the demo graph:

```sql
SELECT
  d.id,
  cast(t.name AS varchar) AS tenant,
  cast(a.name AS varchar) AS author,
  cast(d.status AS varchar) AS status,
  d.score
FROM documents d
JOIN tenants t ON d.tenant_id = t.id
JOIN authors a ON d.author_id = a.id
ORDER BY d.score DESC;
```

Press Ctrl/Command+Enter to run. Use **Copy CSV** to copy the current capped
result.

### 5. ANN

Select:

```text
Table: documents
Query: hybrid retrieval across indexes
k: 3
Minimum score: 0.25
```

Click **Search**. Direct mode first attempts native `retrieve_text`; successful
native results show provider, model, dimension, fingerprint, and registry
generation. Otherwise Viewer falls back to exact ANN SQL.

Search is only over the selected table. It is not a cross-table or
whole-database search.

### 6. Agent

Configure an endpoint only if you intend to send data to it. The model must
support OpenAI-style Chat Completions and tool calls.

Try:

```text
Describe the documents table and count documents by status.
```

The Agent may call schema and SQL tools. It can also call mutating tools, so use
a disposable demo while learning. **Save** persists URL/model settings; the API
key remains in process memory. Read [Agent](agent.md) first.

### 7. MCP

Keep the demo connected. Open **MCP**, retain port `7337`, and click
**Start MCP**.

Test health:

```sh
curl http://127.0.0.1:7337/health
```

Stop MCP after the test. See [MCP](mcp.md) for client configuration, stdio, and
the complete tool contract.

### 8. About

Open **About**, then **Licenses** and **Credits**. These documents are bundled
into the binary and work offline.

## Rail and shortcuts

| Key | Page or action | Connection required |
| --- | --- | :---: |
| `1` | Deck | Yes |
| `2` | Stars | Yes |
| `3` | Table | Yes |
| `4` | SQL | Yes |
| `5` | ANN | Yes |
| `6` | Agent | Yes |
| `7` | MCP | Yes |
| `8` or `0` | About | No |
| Ctrl/Command+F | Toggle command palette | No |
| Ctrl/Command+Enter | Run SQL while editor is focused | Yes |
| `?` | Toggle shortcut help when not typing | No |

The command palette includes navigation, tables, sample queries, schema-derived
recipes, REINDEX actions, and disconnect. Use Arrow Up/Down, Enter, and Escape
inside the palette.

## Refresh and disconnect

- **Sync** reloads overview and graph metadata.
- SQL from the workbench refreshes overview and insights after it completes.
- Click the path chip in the top bar and confirm **Disconnect** to release the
  Direct lock.
- Stop MCP separately. Disconnect does not stop its HTTP listener.

## Next steps

- Open real data: [Connections](connections.md)
- Learn query limits and result rendering: [SQL](sql.md)
- Choose an ANN backend: [Vector search](ann.md)
- Review local state and secrets: [Operations](operations.md)
- Understand internals: [Architecture](architecture.md)
