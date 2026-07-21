# Vector search (ANN)

**ANN** installs approximate nearest-neighbor indexes and runs **semantic search**
on a single table. It does **not** search the entire database.

## Scope

| Action | Scope |
| ------ | ----- |
| Install ANN | One table + embedding column (default `embedding`) |
| Semantic search | One table’s ANN surface only |

Pick the table in the **Table** dropdown before install or search.

## Algorithm × quantization (MongrelDB 0.63+)

Algorithm (graph/structure) and quantization (vector representation) are
**separate** schema fields. Only these pairs are supported:

| Algorithm | Quantizations |
| --------- | ------------- |
| **`hnsw`** (default) | `dense`, `binary_sign`, `product` |
| **`diskann`** (Vamana) | `dense` |
| **`ivf`** | `dense` |

| Quantization | Meaning |
| ------------ | ------- |
| **`dense`** (default in Viewer) | Full f32 vectors with cosine distance |
| **`binary_sign`** | Legacy compact Hamming / sign-bit path (HNSW only) |
| **`product`** | Product quantization (PQ codes, ADC distance). Requires `num_subvectors` that evenly divides the column dimension. Product keeps `algorithm = 'hnsw'` as its compatibility selector while executing on a flat PQ backend. |

The install UI defaults to **HNSW × Dense**. DiskANN/IVF coerce quantization to
Dense. Product needs a `num_subvectors` value (e.g. **48** for MiniLM 384-d).

SQL examples:

```sql
-- HNSW dense (Viewer default)
CREATE INDEX docs_ann ON documents USING ann (embedding)
  WITH (m = 16, ef_construction = 64, ef_search = 64,
        algorithm = 'hnsw', quantization = 'dense');

-- DiskANN dense
CREATE INDEX docs_ann ON documents USING ann (embedding)
  WITH (algorithm = 'diskann', quantization = 'dense',
        diskann_r = 64, diskann_l = 128, beam_width = 8);

-- IVF dense
CREATE INDEX docs_ann ON documents USING ann (embedding)
  WITH (algorithm = 'ivf', quantization = 'dense', nlist = 256, nprobe = 8);

-- Product quantization (HNSW selector)
CREATE INDEX docs_ann ON documents USING ann (embedding)
  WITH (algorithm = 'hnsw', quantization = 'product',
        num_subvectors = 48, bits_per_subvector = 8);
```

Omitting quantization on older engines defaulted to BinarySign — Viewer always
sends explicit `algorithm` and `quantization`.

## Eligibility (Enable)

**Enable 384-d … + embed with MiniLM** is available only when:

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
- **Rebuild as … ANN** — `DROP INDEX` + `CREATE INDEX` with the selected
  algorithm/quantization (and re-embed if a text column is selected). Use this to
  change backends (e.g. BinarySign → Dense, HNSW → DiskANN).

Active status shows algorithm, quantization, `m`, `ef_construction`, `ef_search`,
and backend-specific knobs (DiskANN R/L/beam, IVF nlist/nprobe, PQ subvectors).

## Install flow

1. Choose a table.  
2. Choose **algorithm** (HNSW recommended).  
3. Choose **quantization** (Dense recommended).  
4. For Product, set **num_subvectors** (must divide dimension).  
5. Choose a **text column** that actually exists (e.g. `body` on documents,
   `payload` or `kind` on events).  
6. Click **Enable 384-d … + embed with MiniLM**.  
7. Wait for local MiniLM load (first run may download the model into the user
   cache).  

Default dimension is **384** (`all-MiniLM-L6-v2`). Application-supplied vectors
are written into the embedding column after MiniLM runs in-process.

## Semantic search

1. Ensure the table is vector ready.  
2. Enter a natural-language **query**.  
3. Set **k** (max hits) and optional **min cosine score** (0 = off).  
4. Click **Search (ANN + exact rerank)**.  

The engine path prefers exact cosine rerank (`ann_search_exact`) and can fall
back to `ann_search` when needed. Hits may include score columns such as
`exact_score` and `search_rank`. On **Dense** indexes the graph uses cosine
distance; on **BinarySign** HNSW prefilters with Hamming and exact rerank
restores cosine order; **Product** reports ADC distance (with optional engine
reconstructed-vector rerank).

### Interpreting results

- **Top-k** always means “up to k neighbors,” not “all relevant rows.”  
- With k equal to the table size you will often see every row unless min score
  filters weak matches.  
- Unrelated queries should score lower; raise min score to drop them.  

## Schema transparency

Table view and constellation show:

- ANN options: algorithm, quantization, `m`, `ef_construction`, `ef_search`,
  and backend-specific fields  
- Embedding column **source** when the schema records one (`supplied_by_application`,
  `configured_model · provider / model @ version`, etc.)  

## Rebuild vs REINDEX

| Action | What it does |
| ------ | ------------ |
| **Rebuild ANN** (this tab) | Drops the ANN index and recreates it (optionally re-embeds). Changes algorithm/quantization. Direct only. |
| **REINDEX table / all** (Table tab) | Engine maintenance: analyze + compact + GC on one table or the whole DB. Does **not** change ANN options. |

SQL equivalents:

```sql
-- Rebuild ANN (Viewer also does this via the Rebuild button)
DROP INDEX docs_ann ON documents;
CREATE INDEX docs_ann ON documents USING ann (embedding)
  WITH (m = 16, ef_construction = 64, ef_search = 64,
        algorithm = 'hnsw', quantization = 'dense');

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
- Unsupported algorithm × quantization pairs are rejected fail-closed by the
  engine and by the Viewer.  

Related: [SQL](sql.md) · [Table](table.md) · [Onboarding](onboarding.md)
