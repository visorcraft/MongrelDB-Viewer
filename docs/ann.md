# Vector search (ANN)

**ANN** installs HNSW indexes and runs **semantic search** on a single table.
It does **not** search the entire database.

## Scope

| Action | Scope |
| ------ | ----- |
| Install ANN | One table + embedding column (default `embedding`) |
| Semantic search | One table’s ANN surface only |

Pick the table in the **Table** dropdown before install or search.

## Quantization (Dense vs BinarySign)

MongrelDB 0.62+ exposes two ANN quantizations (algorithm is always **HNSW**):

| Value | Meaning |
| ----- | ------- |
| **`dense`** (default) | Full f32 vectors with cosine distance in the graph |
| **`binary_sign`** | Legacy compact Hamming / sign-bit path |

The install UI defaults to **Dense**. Choose **BinarySign** only as an advanced
option when you want the compact representation.

SQL equivalent:

```sql
CREATE INDEX docs_ann ON documents USING ann (embedding)
  WITH (m = 16, ef_construction = 64, ef_search = 64, quantization = 'dense');
```

Use `quantization = 'binary_sign'` for the legacy path. Omitting quantization
on older engines defaulted to BinarySign — Viewer always sends an explicit value.

## Eligibility (Enable)

**Enable 384-d Dense ANN + embed with MiniLM** is available only when:

- Connection mode is **Direct** (not server)  
- Table schema has loaded  
- The table has at least one **embeddable text** column (Bytes / JSON / string
  family - not pure int/float)  
- You selected a valid text column  

If the table is not eligible, Enable stays disabled with a **Not eligible**
reason. Server mode requires installing ANN via server-side SQL / tooling
instead of this button.

Tables already showing **vector ready** / **Active** do not re-offer Enable;
use:

- **Re-embed from text column** — rewrite vectors only (same index).  
- **Rebuild as Dense/BinarySign ANN** — `DROP INDEX` + `CREATE INDEX` with the
  selected quantization (and re-embed if a text column is selected). Use this to
  upgrade a legacy BinarySign index to Dense.

Active status shows the stored quantization, `m`, `ef_construction`, and
`ef_search`.

## Install flow

1. Choose a table.  
2. Choose **quantization** (Dense recommended).  
3. Choose a **text column** that actually exists (e.g. `body` on documents,
   `payload` or `kind` on events).  
4. Click **Enable 384-d … ANN + embed with MiniLM**.  
5. Wait for local MiniLM load (first run may download the model into the user
   cache).  

Default dimension is **384** (`all-MiniLM-L6-v2`). Application-supplied vectors
are written into the embedding column after MiniLM runs in-process.

## Semantic search

1. Ensure the table is vector ready.  
2. Enter a natural-language **query**.  
3. Set **k** (max hits) and optional **min cosine score** (0 = off).  
4. Click **Search (HNSW + exact rerank)**.  

The engine path prefers exact cosine rerank (`ann_search_exact`) and can fall
back to `ann_search` when needed. Hits may include score columns such as
`exact_score` and `search_rank`. On **Dense** indexes the graph itself uses
cosine distance; on **BinarySign** HNSW prefilters with Hamming and exact
rerank restores cosine order.

### Interpreting results

- **Top-k** always means “up to k neighbors,” not “all relevant rows.”  
- With k equal to the table size you will often see every row unless min score
  filters weak matches.  
- Unrelated queries should score lower; raise min score to drop them.  

## Schema transparency

Table view and constellation show:

- ANN options: quantization, `m`, `ef_construction`, `ef_search`  
- Embedding column **source** when the schema records one (`supplied_by_application`,
  `configured_model · provider / model @ version`, etc.)  

## Rebuild vs REINDEX

| Action | What it does |
| ------ | ------------ |
| **Rebuild ANN** (this tab) | Drops the ANN index and recreates it (optionally re-embeds). Changes quantization. Direct only. |
| **REINDEX table / all** (Table tab) | Engine maintenance: analyze + compact + GC on one table or the whole DB. Does **not** change ANN quantization. |

SQL equivalents:

```sql
-- Rebuild ANN (Viewer also does this via the Rebuild button)
DROP INDEX docs_ann ON documents;
CREATE INDEX docs_ann ON documents USING ann (embedding)
  WITH (m = 16, ef_construction = 64, ef_search = 64, quantization = 'dense');

-- Table / database maintenance
REINDEX documents;
REINDEX;
```

## Tips

- Wrong text column (e.g. `body` on `events`) is blocked or fails with a clear
  column list - use the dropdown.  
- Re-open the same Direct root later: ANN remains in the database schema.  
- Server connections: run search if the server already has ANN; install/rebuild
  from Direct or admin SQL. REINDEX may work over server SQL if the daemon
  allows it.  
- Product quantization and non-HNSW algorithms are **not** implemented in the
  engine — do not expect other ANN backends here.  

Related: [SQL](sql.md) · [Table](table.md) · [Onboarding](onboarding.md)
