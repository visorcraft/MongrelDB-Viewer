# Credits and Attribution

## Copyright

MongrelDB Viewer is © VisorCraft LLC and contributors, dual-licensed under
the [MIT License](LICENSE-MIT) OR the [Apache License 2.0](LICENSE-APACHE).

## Product

MongrelDB Viewer is a multi-platform **Signal Deck** for exploring
AI-native [MongrelDB](https://github.com/visorcraft/mongreldb) databases.
It embeds `mongreldb-core` / `mongreldb-query` for exclusive local opens and
uses `mongreldb-client` for multi-client HTTP access to `mongreldb-server`.

Source repository: https://github.com/visorcraft/MongrelDB-Viewer

## Runtime dependencies

On Linux desktops the UI shell links against system libraries provided by
the host (WebKitGTK, GTK, GLib, and related stacks). Downstream packagers
(AppImage, distro packages) handle redistribution of those shared objects.
Full license texts for major runtime components are available under the
Licenses page **Runtime components** tab and summarized in Credits.

| Component | Typical license | Project |
| --------- | --------------- | ------- |
| WebKitGTK / WRY | LGPL-2.1+ / BSD | https://webkitgtk.org |
| GTK 3 | LGPL-2.1+ | https://www.gtk.org |
| GLib / GObject | LGPL-2.1+ | https://docs.gtk.org/glib/ |
| Cairo | LGPL-2.1 / MPL-1.1 | https://www.cairographics.org |
| libsoup | LGPL-2.1+ | https://libsoup.org |
| OpenSSL (if used by system TLS) | Apache-2.0 | https://www.openssl.org |
| glibc / libstdc++ | LGPL-2.1+ / GPLv3 with runtime exception | GNU |

On Windows and macOS, equivalent platform WebView runtimes apply
(WebView2 / WKWebView) under their vendor terms.

## Rust crate dependencies

Direct application dependencies (versions resolved in `src-tauri/Cargo.lock`;
regenerate inventories with `scripts/regen-credits.sh`).

### Application shell

| Crate | Role | License |
| ----- | ---- | ------- |
| `tauri` | Desktop shell | Apache-2.0 OR MIT |
| `tauri-build` | Build-time Tauri codegen | Apache-2.0 OR MIT |
| `tauri-plugin-opener` | Open URLs / paths | Apache-2.0 OR MIT |
| `tauri-plugin-dialog` | Native file dialogs | Apache-2.0 OR MIT |
| `serde`, `serde_json` | Serialization | MIT OR Apache-2.0 |
| `tokio`, `tokio-util`, `futures` | Async runtime | MIT / MIT OR Apache-2.0 |
| `reqwest` | HTTP (chat + remote embeddings) | MIT OR Apache-2.0 |
| `axum`, `tower-http` | Local MCP HTTP bridge | MIT |
| `parking_lot`, `dashmap` | Concurrency helpers | MIT OR Apache-2.0 / MIT |
| `dirs`, `uuid`, `chrono` | Paths / IDs / time | MIT OR Apache-2.0 |
| `thiserror`, `tracing`, `bytes`, `base64`, `async-trait` | Utilities | MIT OR Apache-2.0 |

### MongrelDB engine

| Crate | Role | License |
| ----- | ---- | ------- |
| `mongreldb-core` 0.64.5 | Embedded storage engine | MIT OR Apache-2.0 |
| `mongreldb-query` 0.64.5 | SQL / DataFusion query layer | MIT OR Apache-2.0 |
| `mongreldb-client` 0.64.5 | HTTP client for mongreldb-server | MIT OR Apache-2.0 |
| `mongreldb-kit` 0.64.5 | Kit schema sidecar for demo roots | MIT OR Apache-2.0 |
| `arrow` | Columnar batches | Apache-2.0 |

### Embeddings

| Crate | Role | License |
| ----- | ---- | ------- |
| `fastembed` (optional feature `local-embeddings`) | Local MiniLM dense embeddings | Apache-2.0 |

The complete transitive crate list, versions, and full license texts are
available in-app under **Licenses → Third-party (Rust)** and as a filterable
table under **Credits**.

## Frontend (npm)

Direct packages from root `package.json` (versions resolved in
`package-lock.json`).

### Runtime (shipped UI)

| Package | Version | Role | License |
| ------- | ------- | ---- | ------- |
| `react` | 19.2.7 | UI | MIT |
| `react-dom` | 19.2.7 | UI DOM renderer | MIT |
| `@tauri-apps/api` | 2.11.1 | Frontend bridge | Apache-2.0 OR MIT |
| `@tauri-apps/plugin-dialog` | 2.7.2 | Dialog plugin client | MIT OR Apache-2.0 |
| `@tauri-apps/plugin-opener` | 2.5.4 | Opener plugin client | MIT OR Apache-2.0 |

Transitive runtime package used by React:

| Package | Version | Role | License |
| ------- | ------- | ---- | ------- |
| `scheduler` | 0.27.0 | React scheduling | MIT |

### Build tooling (devDependencies)

| Package | Version | Role | License |
| ------- | ------- | ---- | ------- |
| `@tauri-apps/cli` | 2.11.4 | Tauri CLI | Apache-2.0 OR MIT |
| `vite` | 8.1.5 | Bundler / dev server | MIT |
| `@vitejs/plugin-react` | 6.0.3 | React plugin for Vite | MIT |
| `typescript` | 5.9.3 | Type checking | Apache-2.0 |
| `@types/react` | 19.2.17 | Type definitions | MIT |
| `@types/react-dom` | 19.2.3 | Type definitions | MIT |

The complete installed npm tree (runtime + build tooling), versions, and full
license texts are available in-app under **Licenses → Frontend (npm)** and as a
filterable table under **Credits**.


## Contact

License questions: https://www.visorcraft.com
