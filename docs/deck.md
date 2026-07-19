# Deck (Overview)

**Deck** is the home dashboard after you connect. It summarizes the open
database and jumps into exploration.

## What you see

### Stat cards

Typical cards include:

- **Tables** - table count  
- **Rows** - sum of table row counts (when available)  
- **ANN tables** - how many tables already have dense ANN  

Exact cards can vary with connection mode and engine metadata.

### Try these

When the app can infer safe, schema-driven recipes (especially on the demo
database), chips appear for common explorations - for example open a table,
run a sample `SELECT`, or jump to vector search. Clicking a recipe either
changes page or fills the SQL console.

### Tables panel

A roster of tables with:

- name  
- row count  
- column count  
- index count  
- capability chips (Bitmap, Range, Text, ANN, Sparse, MinHash, …)  

Click a table name (or open action) to go to **Table** for that relation.

The panel shows Direct vs Server mode. Disconnect lives in the **top bar**, not
on this card.

## Refresh

Use **Sync** on the rail to reload overview, insights, and related metadata
after DDL or ANN install.

## Tips

- Prefer Deck after reconnecting to confirm which root or server URL is open.  
- Capability chips are a quick ANN readiness check before Vector search.  

Related: [Table](table.md) · [Schema map](constellation.md) · [Onboarding](onboarding.md)
