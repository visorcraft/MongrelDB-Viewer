# SQL workbench

**SQL** runs MongrelDB / DataFusion SQL against the open connection (direct
embed or server).

## Editor

- Type or paste a statement in the editor  
- **Run** executes with a configurable max row cap  
- **Ctrl/⌘+Enter** runs when the page is focused  

## Results

Successful statements show:

- result grid (columns × rows)  
- row count, elapsed time, statement kind  
- truncation notice when max rows is hit  

Errors appear in the red banner at the top of the content area.

## History and suggestions

- Recent statements are kept in a short in-app history list  
- Schema-driven **suggested queries** may appear when insights are available
  (demo and many real schemas)  

Click a history or suggestion entry to load it into the editor.

## What works well

- `SELECT` exploration and joins  
- DDL/DML supported by the engine  
- Engine scored-search table functions when you prefer raw SQL over the ANN UI  

## What to avoid

- Pasting secrets into SQL comments or string literals that you might screenshot  
- Unbounded scans on huge tables without `LIMIT`  

Related: [Table](table.md) · [Vector search](ann.md) · [Agent](agent.md)
