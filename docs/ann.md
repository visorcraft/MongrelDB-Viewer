# Vector search (ANN)

**ANN** installs dense HNSW indexes and runs **semantic search** on a single
table. It does **not** search the entire database.

## Scope

| Action | Scope |
| ------ | ----- |
| Install Dense ANN | One table + embedding column (default `embedding`) |
| Semantic search | One table’s ANN surface only |

Pick the table in the **Table** dropdown before install or search.

## Eligibility (Enable)

**Enable 384-d ANN + embed with MiniLM** is available only when:

- Connection mode is **Direct** (not server)  
- Table schema has loaded  
- The table has at least one **embeddable text** column (Bytes / JSON / string
  family - not pure int/float)  
- You selected a valid text column  

If the table is not eligible, Enable stays disabled with a **Not eligible**
reason. Server mode requires installing ANN via server-side SQL / tooling
instead of this button.

Tables already showing **vector ready** / **Active** do not re-offer Enable;
use **Re-embed from text column** when you need to refresh vectors.

## Install flow

1. Choose a table.  
2. Choose a **text column** that actually exists (e.g. `body` on documents,
   `payload` or `kind` on events).  
3. Click **Enable 384-d ANN + embed with MiniLM**.  
4. Wait for local MiniLM load (first run may download the model into the user
   cache).  

Default dimension is **384** (`all-MiniLM-L6-v2`).

## Semantic search

1. Ensure the table is vector ready.  
2. Enter a natural-language **query**.  
3. Set **k** (max hits) and optional **min cosine score** (0 = off).  
4. Click **Search (HNSW + exact rerank)**.  

The engine path prefers exact cosine rerank (`ann_search_exact`) and can fall
back to `ann_search` when needed. Hits may include score columns such as
`exact_score` and `search_rank`.

### Interpreting results

- **Top-k** always means “up to k neighbors,” not “all relevant rows.”  
- With k equal to the table size you will often see every row unless min score
  filters weak matches.  
- Unrelated queries should score lower; raise min score to drop them.  

## Tips

- Wrong text column (e.g. `body` on `events`) is blocked or fails with a clear
  column list - use the dropdown.  
- Re-open the same Direct root later: ANN remains in the database schema.  
- Server connections: run search if the server already has ANN; install from
  Direct or admin SQL.  

Related: [SQL](sql.md) · [Table](table.md) · [Onboarding](onboarding.md)
