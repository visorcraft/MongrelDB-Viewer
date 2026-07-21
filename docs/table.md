# Table browser

**Table** inspects a single relation: columns, indexes, and optional sample
access.

## Selecting a table

- Pick from the table selector on the page  
- Click a table from **Deck** or **Schema map**  
- Use the command palette to jump by name  

## Columns

For each column you typically see:

- name  
- type (including Embedding dimensions when present)  
- flags (primary key, nullable, …)  
- embedding source metadata when applicable  

## Index radar

A summary of secondary index kinds on the table:

| Kind | Role |
| ---- | ---- |
| Bitmap | Equality / low-cardinality filters |
| Learned range (PGM) | Numeric / range filters |
| FM | Substring / text containment |
| ANN | Dense vector nearest neighbor |
| Sparse | Learned-sparse retrieval |
| MinHash | Set similarity / near-dup |

## Indexes list

Named secondary indexes with kind and target column.

## Sample / SQL

Actions may open the SQL workbench with a `SELECT … LIMIT` for the current
table so you can inspect rows without leaving the explorer flow.

## Maintenance: REINDEX

From the Table inspector:

| Button | SQL equivalent | Effect |
| ------ | -------------- | ------ |
| **REINDEX table** | `REINDEX <table>` | Analyze + compact this table, then GC |
| **REINDEX all** | `REINDEX` | Same for every table |

Also available from the command palette under **Maintenance**.

This is **index maintenance**, not ANN rebuild. To change ANN quantization
(e.g. BinarySign → Dense, HNSW → DiskANN), use **Rebuild ANN** on the Vector search tab.

## Tips

- Tables without ANN are still fully browsable - vector search simply will not
  enable until you install ANN (when eligible).  
- Large tables: prefer SQL with filters and limits instead of unbounded samples.  

Related: [Deck](deck.md) · [SQL](sql.md) · [Vector search](ann.md)
