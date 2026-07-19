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

MongrelDB Viewer pulls crates from crates.io, including:

### Application shell

| Crate | Role | License (typical) |
| ----- | ---- | ----------------- |
| `tauri`, `tauri-plugin-opener`, `tauri-plugin-dialog` | Desktop shell | Apache-2.0 OR MIT |
| `serde`, `serde_json` | Serialization | Apache-2.0 OR MIT |
| `tokio`, `futures` | Async runtime | MIT |
| `reqwest` | HTTP (chat + remote embeddings) | Apache-2.0 OR MIT |
| `axum`, `tower-http` | Local MCP HTTP bridge | MIT |
| `parking_lot`, `dashmap`, `dirs`, `uuid`, `chrono`, `thiserror`, `tracing`, `bytes`, `base64`, `async-trait` | Utilities | MIT / Apache-2.0 |

### MongrelDB engine

| Crate | Role | License (typical) |
| ----- | ---- | ----------------- |
| `mongreldb-core` | Embedded storage engine | See crate |
| `mongreldb-query` | SQL / DataFusion query layer | See crate |
| `mongreldb-client` | HTTP client for mongreldb-server | See crate |
| `arrow` | Columnar batches | Apache-2.0 |

### Embeddings

| Crate | Role | License (typical) |
| ----- | ---- | ----------------- |
| `fastembed` (optional feature) | Local MiniLM dense embeddings | Apache-2.0 |

The complete transitive crate list, versions, and full license texts are
available in-app under **Licenses → Third-party** and as a filterable
table under **Credits**.

## Frontend (npm)

| Package | Role | License |
| ------- | ---- | ------- |
| `react`, `react-dom` | UI | MIT |
| `@tauri-apps/api` / plugins | Frontend bridge | Apache-2.0 OR MIT |
| `vite`, `typescript` (dev) | Build tooling | MIT |

## Contact

License questions: https://www.visorcraft.com
