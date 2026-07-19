# Schema map (Stars)

**Stars** shows a pan/zoom **constellation** of tables, foreign keys, and index
signals for the open database.

## Layout

- **Nodes** - tables (and sometimes related structural nodes)  
- **Edges** - foreign-key relationships (often dashed / colored)  
- **Hints** - hover or select for table names and counts  

## Controls

- **Pan** - drag the canvas background  
- **Zoom** - scroll / trackpad (when supported by the canvas)  
- **Click a table** - opens **Table** browser for that name  
- Fit / reset behavior refits when the graph identity changes (new DB), not on
  every pan  

## When to use it

- Understand multi-table demos (tenants → authors/documents → events, tags, …)  
- Spot which tables participate in FK graphs before writing join SQL  
- Orient after connecting to an unfamiliar server database  

## Empty state

If no schema is loaded, connect a database first from Welcome.

Related: [Deck](deck.md) · [Table](table.md) · [SQL](sql.md)
