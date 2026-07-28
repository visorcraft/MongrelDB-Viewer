# Table browser and maintenance

**Table** combines schema inspection, index details, live row sampling, and
REINDEX controls for one relation.

![Table browser](images/03-table.png)

## Select a table

Open a table from:

- the selector in Table,
- a Deck table card,
- a Stars table node,
- Ctrl/Command+F command palette.

Changing the selection reloads table detail and automatically requests live
rows. A new table resets the projection to all columns so a prior table's
column list cannot leak into its query. The initial row limit is 50; later
selections keep the current limit.

## Inspector

The header reports:

```text
schema_id
row count
column count
```

Each column row includes:

| Field | Meaning |
| --- | --- |
| ID | Engine column identifier |
| Name | SQL/schema name |
| Type | MongrelDB type, including `Embedding(<dimension>)` |
| Flags | `PRIMARY_KEY`, `NULLABLE`, `ENCRYPTED`, `AUTO_INCREMENT`, and embedding quantization flags when present |
| Embedding source | Application-supplied or configured provider/model metadata |

When an embedding column has no explicit source metadata, Viewer labels it
`supplied_by_application`.

Server mode receives the flags exposed by Kit schema: primary key, nullable,
and auto increment. Direct mode can expose the fuller core flag set.

## Index radar

The radar counts index entries on this table:

| Family | Intended signal |
| --- | --- |
| Bitmap | Equality and low-cardinality filtering |
| LearnedRange | Numeric/range access using PGM-style modeling |
| FM-index | Text and substring access |
| ANN | Dense vector nearest-neighbor access |
| Sparse | Learned sparse retrieval |
| MinHash | Set similarity and near-duplicate access |

Unlike Deck's capability radar, this radar counts individual indexes.

## Index list

Each index row shows:

- name,
- family,
- covered column,
- option summary,
- live semantic identity when available.

Option summaries can include:

| Family | Displayed options |
| --- | --- |
| ANN | algorithm, quantization, `m`, `efc`, `efs`, and backend-specific parameters |
| MinHash | permutations and bands |
| LearnedRange | epsilon |

Direct ANN inspection may show provider, model, version, and a short
fingerprint from the live index. Server mode does not hydrate live semantic
identity.

Older ANN schemas with omitted options are displayed using the engine legacy
default `hnsw · binary_sign (default)`. Viewer-created ANN always writes
algorithm and quantization explicitly.

## Live rows

The lower panel executes:

```sql
SELECT <projection> FROM <table> LIMIT <row-limit>
```

Available row limits:

```text
25, 50, 100, 250, 500
```

Projection controls:

- **All columns** selects `*`.
- **Hide embeddings** selects every non-embedding column.
- Column chips show up to eight non-embedding names for orientation.
- **Refresh** executes the currently selected projection and limit.
- **Open in SQL** copies the query into the SQL workbench without running it.

Changing All/Hide projection does not itself reload rows. Click **Refresh**.

Large f32 embedding values render as a compact object containing dimension and
a first/last-four preview. Use **Hide embeddings** to reduce transfer and
rendering cost.

## Sample rows

The inspector's **Sample rows** button switches to SQL and immediately runs:

```sql
SELECT * FROM <table> LIMIT 50
```

This differs from **Open in SQL**, which prepares but does not execute the
query.

## REINDEX

| Button | SQL equivalent | Scope |
| --- | --- | --- |
| **REINDEX table** | `REINDEX <table>` | Selected table |
| **REINDEX all** | `REINDEX` | Entire database |

REINDEX performs engine maintenance:

```text
analyze + compact + garbage collection
```

It can be expensive and is not read-only maintenance. Run it from a tested
backup posture and avoid overlapping heavy writes.

Direct mode calls the local query session. Server mode sends the SQL statement
to the daemon, where authorization and server support decide success. Table
names passed through the dedicated action are limited to ASCII letters,
digits, and underscore.

REINDEX does not change ANN algorithm, quantization, or embedding vectors. Use
**ANN** -> **Rebuild** to replace an ANN index. Rebuild drops and recreates the
index; REINDEX maintains the current one.

## Stale or failed data

- Click **Sync** after external schema changes.
- If auto-sample fails, inspect schema first and try a smaller projection in
  SQL.
- Server errors prefixed `schema <table>:` come from Kit schema loading.
- Server errors prefixed `count <table>:` come from the count endpoint.
- A missing semantic identity does not mean the ANN index is absent; inspect
  the ANN family and options columns.

Related: [Deck](deck.md) · [SQL](sql.md) · [ANN](ann.md) ·
[Connections](connections.md)
