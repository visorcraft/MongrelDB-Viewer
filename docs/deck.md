# Deck overview

Deck is the landing page after a database connects. It summarizes the current
catalog and turns schema signals into navigation and SQL recipes.

![Deck overview](images/01-deck.png)

## Header

The hero shows:

- Direct path or Server URL,
- connection mode,
- linked engine and query versions,
- MongrelDB git SHA,
- session uptime,
- quick links to Stars, SQL, and ANN.

In Server mode, the displayed engine/query build comes from the crates linked
into Viewer, not an authoritative remote-daemon version endpoint.

## Summary cards

| Card | Source |
| --- | --- |
| Tables | Number of tables in the loaded overview |
| Rows | Sum of each table's count |
| Secondary indexes | Sum of schema secondary-index entries |

Direct counts come from embedded table handles. Server counts require a request
per table and can be slower.

ANN-ready table count appears as a hero pill and a generated Insight card.

## Index radar

The radar counts how many tables advertise each public index family:

- Bitmap,
- LearnedRange/PGM,
- FM/text,
- ANN,
- Sparse,
- MinHash.

It counts table capabilities, not index entries. A table with several Bitmap
indexes contributes one Bitmap-capable table to the Deck radar.

## Insights

Viewer derives insights from the current schema. It does not depend on demo
table names.

The generator recognizes:

- Bitmap and categorical columns for group counts,
- numeric and score-like columns for ranges and sort recipes,
- timestamp-like columns for recent-row recipes,
- Bytes/text/JSON columns for substring recipes,
- nullable columns for presence filters,
- embedding columns and ANN readiness.

During connect, it may run up to four small group-by probes to populate live
cards. A server with expensive scans may therefore take longer to build Deck.
Clicking an enabled Insight card opens and runs its SQL. Disabled cards have no
query.

## Recipes

Deck and SQL share at most 28 unique schema-derived suggestions. Categories
include:

```text
catalog
browse
stats
filter
search
```

Deck's **Try these** panel shows the first ten. SQL exposes the full generated
list with category filters. Recipes are starting points, not a safety boundary.
Review generated SQL before using it on important data.

## Tables panel

Each table row shows:

- row count,
- column count,
- secondary-index count,
- capability chips,
- embedding dimensions when present.

Click the table name to open **Table**. Click **Preview** to run:

```sql
SELECT * FROM <table> LIMIT 25
```

Use Table's **Hide embeddings** projection for wide vector columns.

## Refresh behavior

Use **Sync** on the rail after:

- DDL from another client,
- server-side schema work,
- external writes that change counts,
- ANN maintenance outside Viewer.

Running SQL through Viewer's workbench refreshes overview and insights after
the request. ANN and REINDEX actions also refresh relevant metadata.

Sync reloads the catalog and graph. It does not reconnect, restart MCP, or
change Agent state.

## Navigation and disconnect

- Click the helmet to return to Deck.
- Ctrl/Command+F opens the command palette.
- Key `1` opens Deck while connected.
- Click the path/URL chip and confirm **Disconnect** to close the active
  database handle.

Disconnect does not stop an MCP listener already started from this process.

Related: [First launch](onboarding.md) · [Schema map](constellation.md) ·
[Table](table.md) · [SQL](sql.md)
