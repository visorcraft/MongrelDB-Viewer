# MongrelDB Viewer documentation

MongrelDB Viewer is a desktop client for exploring and operating an open
MongrelDB database. These guides cover installation, every UI surface, Agent
and MCP integrations, data persistence, security, troubleshooting, and
repository architecture.

## Start

| Guide | Use it for |
| --- | --- |
| [Installation and builds](installation.md) | Release packages, source prerequisites, development, bundles, model cache, upgrades |
| [First launch and demo](onboarding.md) | Create the demo, learn navigation, run the first queries |
| [Connection modes](connections.md) | Direct locks and credentials, Server auth and limitations, recents, disconnect |

Fast path:

```sh
git clone https://github.com/visorcraft/MongrelDB-Viewer.git
cd MongrelDB-Viewer
npm ci
npm run tauri dev
```

Choose an empty directory and click **Create demo DB**.

## Use every surface

| Surface | Guide | Main operations |
| --- | --- | --- |
| Deck | [Overview](deck.md) | Counts, capabilities, insights, recipes, refresh |
| Stars | [Schema map](constellation.md) | Graph nodes, edges, pan, zoom, mode differences |
| Table | [Table browser](table.md) | Columns, flags, indexes, row samples, REINDEX |
| SQL | [SQL workbench](sql.md) | Execution, caps, history, recipes, result conversion |
| ANN | [Vector search](ann.md) | Install, rebuild, backfill, algorithms, scoring, provenance |
| Agent | [Agent chat](agent.md) | Endpoint contract, memory-only key, tool loop, safety, troubleshooting |
| MCP | [MCP bridge](mcp.md) | HTTP, stdio, protocol methods, every tool and argument |
| About | [Licenses and credits](about.md) | Offline legal documents and regeneration |

## Operate safely

| Guide | Contents |
| --- | --- |
| [Configuration, local data, and troubleshooting](operations.md) | Persistent keys, cache/config paths, environment variables, ports, network activity, errors |
| [Security policy](../SECURITY.md) | Private reporting, supported versions, trust model, credentials, Agent and MCP risks |

Important defaults:

- Direct mode is exclusive and read-write.
- SQL is not restricted to `SELECT`.
- Agent and MCP can run mutating tools without a separate confirmation.
- Agent **Save** persists URL/model settings, never the API key.
- In-app MCP is unauthenticated and bound by the UI to `127.0.0.1`.
- Semantic search is table-scoped, not database-wide.

## Understand and contribute

| Guide | Contents |
| --- | --- |
| [Architecture](architecture.md) | Layers, state, request flows, source map, trust boundaries, internal/public interfaces |
| [Contributing](../CONTRIBUTING.md) | Fork workflow, development gate, tests, dependency/legal updates, release checklist |
| [Compatibility fixture](../src-tauri/tests/fixtures/README.md) | Frozen older root and deliberate regeneration |

## Reference map

```text
Need to install?              installation.md
Cannot connect?               connections.md, then operations.md
Need SQL examples?            sql.md
Need semantic search?         ann.md
Connecting an AI client?      mcp.md
Using an in-app model?        agent.md
Investigating local secrets?  operations.md and SECURITY.md
Changing code?                architecture.md and CONTRIBUTING.md
```

Project: [GitHub](https://github.com/visorcraft/MongrelDB-Viewer) ·
Engine: [MongrelDB](https://github.com/visorcraft/MongrelDB) ·
Product: [mongreldb.com](https://www.mongreldb.com)
