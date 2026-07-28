# Vector search and ANN

**ANN** installs, rebuilds, backfills, and queries one table's dense-vector
surface.

![Vector search](images/05-ann.png)

## Scope and defaults

Viewer's desktop workflow is intentionally fixed around one interoperable
local model:

```text
model:             all-MiniLM-L6-v2
provider id:       viewer-minilm
model version:     1
dimension:         384
new embedding col: embedding
UI backfill:       every eligible row
UI top-k range:    1..100
default top-k:     3
default min score: 0.25
```

Search is scoped to the selected table. It does not search every ANN table,
join related tables, or fuse results across the database.

For a new surface, the desktop page creates `embedding`. For an existing
surface, it uses the first ANN index's actual column and dimension. The page is
not a selector for several ANN columns on one table; use MCP or SQL when that
distinction matters.

The Rust backend supports dimensions from 1 through 4,096 and top-k through
1,000 for internal/MCP requests. New desktop installs use 384. Rebuild and
search preserve the selected existing ANN column's dimension.

## Eligibility

**Enable 384-d ... + embed with MiniLM** requires:

- Direct mode,
- loaded schema,
- no existing ANN index on the table,
- a selected text-like source column.

Eligible source types are names containing Bytes, String, UTF, JSON, Text, or
Char. Numeric, Boolean, and Embedding columns are excluded.

If ANN already exists, the page offers:

- **Re-embed from text column**,
- **Rebuild as ... ANN**.

Server mode can search an existing ANN surface, but install and rebuild controls
are disabled.

## Algorithm and quantization

Algorithm controls index structure. Quantization controls vector
representation. Supported pairs are:

| Algorithm | Dense | BinarySign | Product |
| --- | :---: | :---: | :---: |
| HNSW | Yes | Yes | Yes |
| DiskANN/Vamana | Yes | No | No |
| IVF | Yes | No | No |

### Algorithms

| Algorithm | Use |
| --- | --- |
| `hnsw` | Default in-memory graph and broadest quantization support |
| `diskann` | Vamana/DiskANN path for Dense vectors |
| `ivf` | Inverted-file path for Dense vectors |

### Quantizations

| Quantization | Storage and scoring |
| --- | --- |
| `dense` | Full f32 vectors, cosine distance, recommended default |
| `binary_sign` | Compact sign bits and Hamming prefilter, HNSW only |
| `product` | Product-quantized codes and ADC distance, HNSW compatibility selector |

Product quantization requires:

- `num_subvectors > 0`,
- dimension evenly divisible by `num_subvectors`,
- exactly 8 bits per subvector.

The UI default is 48 subvectors for 384 dimensions.

## Parameters

Viewer's backend defaults:

| Parameter | Default | Applies to |
| --- | ---: | --- |
| `m` | 16 | HNSW-compatible options |
| `ef_construction` | 64 | HNSW-compatible options |
| `ef_search` | 64 | HNSW-compatible options |
| `diskann_r` | Engine default 64 | DiskANN |
| `diskann_l` | Engine default 128 | DiskANN |
| `beam_width` | Engine default 8 | DiskANN |
| `nlist` | Engine default 256 | IVF |
| `nprobe` | Engine default 8 | IVF |
| `num_subvectors` | Required, UI 48 | Product |
| `bits_per_subvector` | 8 | Product |

The desktop page exposes algorithm, quantization, and Product subvectors. MCP
can supply the backend-specific DiskANN and IVF options documented in
[MCP tools](mcp.md#tool-reference).

## Install

1. Select a Direct table.
2. Select an algorithm.
3. Select a supported quantization.
4. For Product, confirm `num_subvectors`.
5. Select a real source text column.
6. Click **Enable 384-d ... + embed with MiniLM**.

The backend:

1. validates the table, source column, dimension, options, and optional
   backfill ceiling,
2. reads every eligible source value by stable internal RowId,
3. embeds chunks of 32 and validates vector count, dimension, and finiteness,
4. validates or durably adds nullable `Embedding(384)` column `embedding`,
5. writes every prepared vector in one core transaction,
6. stamps configured-model source metadata after that commit,
7. creates the ANN index, or drops and recreates it for a rebuild,
8. reports `rowsEmbedded`.

Vector preparation finishes before schema mutation. A missing source, provider
failure, wrong dimension, malformed response, or undersized explicit
`backfill_limit` therefore fails without adding the column or index.

Default generated index name is:

```text
<table>_embedding_ann
```

The demo uses its seeded name `docs_ann`.

Install is durable. The embedding column, vectors, source metadata, and index
remain in the database after disconnect.

### Existing columns

Viewer does not replace a conflicting column:

- `embedding` with the wrong type fails,
- `Embedding(<other-dimension>)` fails,
- an existing compatible `Embedding(384)` is reused.

### Backfill atomicity and limits

Null and empty source values are skipped. The desktop sends no backfill ceiling,
so one action prepares and writes every eligible row. All vector updates commit
in one transaction; one failed update aborts them together.

MCP may pass `backfill_limit` as a preflight safety ceiling. If eligible rows
exceed it, the request fails before embedding or schema changes. Omit the field
to process all eligible rows.

DDL is separate from the vector transaction. A failed initial `CREATE INDEX`
can leave a completely embedded column without ANN. A failed replacement create
after rebuild can leave the previous index dropped. Vectors are not partially
committed.

## Re-embed

**Re-embed from text column**:

- keeps the current index,
- keeps current algorithm and quantization,
- rewrites vectors from the selected source,
- updates configured-model metadata when needed.

Use it when:

- an offline demo received zero vectors,
- source text changed,
- vectors were missing or stale,
- the same 384-dimensional model should recompute them.

It is not an incremental changed-row detector. It rewrites every eligible
vector in one transaction.

## Rebuild

**Rebuild as ... ANN**:

1. finds the first ANN index and its actual embedding column,
2. prepares and atomically writes replacement vectors when a text column is
   selected,
3. runs `DROP INDEX`,
4. runs replacement `CREATE INDEX`.

Rebuild changes algorithm, quantization, or index parameters. It does not drop
the embedding column.

The drop happens before create. If replacement creation fails, vectors remain
but the table can be left without ANN. Back up important roots before rebuild.

## Semantic search

1. Select a vector-ready table.
2. Enter natural-language query text.
3. Choose maximum hits `k`.
4. Set minimum score, or `0` to disable.
5. Click **Search (retrieve_text + ANN)**.

Viewer resolves the provider recorded on the embedding column. A
`supplied_by_application` column has no recorded provider; re-embed it in the
desktop UI or use MCP with an explicit matching `provider_id`.

### Direct preferred path

Direct mode:

1. reads the existing embedding source without modifying it,
2. rejects a provider that conflicts with recorded metadata,
3. registers the matching provider in the process-local registry,
4. calls native `retrieve_text`,
5. hydrates up to ten non-embedding columns,
6. returns rank, score kind, score, row ID, and semantic provenance.

Provenance includes:

- provider ID and version,
- model ID and version,
- dimension,
- first 16 hex characters of model fingerprint,
- provider-registry generation,
- embedding column.

If native retrieval is unavailable, Viewer falls back to SQL using the same
resolved provider. Search never relabels application-supplied vectors.

### Exact SQL fallback

With exact rerank enabled, Viewer asks:

```text
candidate_k = min(max(k * 20, k), 1000)
```

and plans:

```sql
SELECT *
FROM ann_search_exact(
  '<table>',
  '<embedding-column>',
  '<query-vector>',
  <candidate-k>,
  <k>,
  'cosine',
  '<projection>'
);
```

If exact search fails, it falls back to:

```sql
SELECT <projection>
FROM <table>
WHERE ann_search(<embedding-column>, '<query-vector>', <k>);
```

Direct exact projection includes up to eight non-embedding columns. Server
exact projection defaults to `id` unless a caller supplies one.

### Score interpretation

| Result field | Meaning | Better |
| --- | --- | --- |
| `rank` | Returned order | Lower |
| Native `score` with `ann_cosine_distance` | Cosine distance | Lower |
| Native `score` with `ann_hamming_distance` | Hamming distance | Lower |
| SQL `exact_score` | Cosine similarity | Higher |

Minimum score is a cosine-similarity floor:

- SQL exact results keep `exact_score >= threshold`,
- native cosine distance keeps `1 - distance >= threshold`,
- raw fallback results without a recognized exact score are not filtered,
- native Hamming results do not use the cosine floor directly.

Top-k means at most `k` nearest candidates, not every relevant row. If `k`
approaches table size, weak results are normal unless the score floor removes
them.

## SQL DDL examples

HNSW Dense, Viewer's default:

```sql
CREATE INDEX docs_ann ON documents USING ann (embedding)
WITH (
  m = 16,
  ef_construction = 64,
  ef_search = 64,
  algorithm = 'hnsw',
  quantization = 'dense'
);
```

HNSW BinarySign:

```sql
CREATE INDEX docs_ann ON documents USING ann (embedding)
WITH (
  m = 16,
  ef_construction = 64,
  ef_search = 64,
  algorithm = 'hnsw',
  quantization = 'binary_sign'
);
```

DiskANN Dense:

```sql
CREATE INDEX docs_ann ON documents USING ann (embedding)
WITH (
  algorithm = 'diskann',
  quantization = 'dense',
  diskann_r = 64,
  diskann_l = 128,
  beam_width = 8
);
```

IVF Dense:

```sql
CREATE INDEX docs_ann ON documents USING ann (embedding)
WITH (
  algorithm = 'ivf',
  quantization = 'dense',
  nlist = 256,
  nprobe = 8
);
```

Product quantization:

```sql
CREATE INDEX docs_ann ON documents USING ann (embedding)
WITH (
  algorithm = 'hnsw',
  quantization = 'product',
  num_subvectors = 48,
  bits_per_subvector = 8
);
```

Older engine schemas that omitted quantization can resolve to legacy
BinarySign. Viewer always sends explicit algorithm and quantization.

## REINDEX is different

| Action | Changes |
| --- | --- |
| ANN rebuild | Drops/recreates ANN and can change backend or quantization |
| Re-embed | Rewrites vector values without changing index definition |
| `REINDEX <table>` | Analyze, compact, and GC current table/index structures |
| `REINDEX` | Analyze, compact, and GC the full database |

REINDEX does not switch ANN backend or regenerate vectors.

## Direct, Server, Agent, and MCP

| Surface | Install/rebuild | Search path |
| --- | --- | --- |
| Direct ANN page | Yes | Native `retrieve_text`, then SQL fallback |
| Server ANN page | No | Locally embedded query sent as server ANN SQL |
| Agent | Tool can install only on Direct | Shared semantic-search implementation |
| MCP | Tool can install only on Direct | Shared semantic-search implementation |

The current desktop UI has no remote-embedding configuration form. It uses the
local MiniLM provider even when Agent chat points at a remote model.

## Failure checklist

- **Not eligible**: choose a text-like source column or another table.
- **Direct connection required**: reconnect in Direct or administer ANN at the
  server.
- **Wrong dimension/type**: rename or migrate the conflicting column outside
  this workflow.
- **Unsupported pair**: use HNSW with any listed quantization, or Dense with
  DiskANN/IVF.
- **Product divisor error**: use a divisor of 384, such as 48.
- **Model load error**: check network, cache permissions, and build feature.
- **No or weak hits**: verify non-zero vectors, source text, score floor, and
  query/table fit.
- **Server exact function error**: compare server engine support; raw ANN
  fallback is attempted automatically.

Full error help: [Operations](operations.md#troubleshooting).

Related: [SQL](sql.md) · [Table](table.md) · [MCP](mcp.md) ·
[Architecture](architecture.md#ann-installation-flow)
