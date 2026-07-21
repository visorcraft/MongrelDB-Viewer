# Deck (Overview)

**MongrelDB Deck** is the home dashboard after you connect. Click the **helmet**
icon at the top of the left rail anytime to return here.

When no database is open, the rail shows only the helmet and **About**. Deck,
Stars, Table, SQL, ANN, Agent, MCP, and Sync appear after you connect.

## What you see

### Hero (Overview)

Connection label (path or server URL), engine/query versions, git SHA, ANN
readiness, and session uptime. Quick jumps: Schema map, SQL, Vector search
(same ghost style; cyan fill on hover).

### Stat cards

- **Tables** - table count  
- **Rows** - sum of table row counts (when available)  
- **Secondary indexes** - total secondary indexes across tables  

### Index radar

How many tables offer each of the six public index kinds (Bitmap, Range, Text,
ANN, Sparse, MinHash).

### Insights

Schema-driven cards (clickable when they carry a SQL recipe).

### Try these

Safe recipes for this catalog - chips open the SQL console with a ready
statement.

### Tables panel

Roster with row/column/index counts and capability chips. Click a name for
**Table**, or **Preview** for a sample `SELECT`.

Disconnect by clicking the path chip in the **top bar** (confirms first).

## Refresh

Use **Sync** on the rail (connected only) to reload overview and insights after
DDL or ANN install.

## Tips

- Helmet → Overview is the fastest way back after exploring.  
- Capability chips are a quick ANN readiness check before Vector search.  

Related: [Table](table.md) · [Schema map](constellation.md) · [Onboarding](onboarding.md)
