# Installation and builds

MongrelDB Viewer is a Tauri 2 desktop application. The frontend is React and
TypeScript; the database, SQL, embedding, Agent, and MCP work run in Rust.

## Choose an installation path

| Goal | Recommended path |
| --- | --- |
| Use a published build | Download a matching asset from [GitHub Releases](https://github.com/visorcraft/MongrelDB-Viewer/releases) |
| Try or contribute to current source | Run `npm run tauri dev` |
| Produce a local package | Run `npm run tauri build` |
| Run only headless MCP | Build once, then launch the binary with `--mcp-stdio` |

Release assets vary by release and platform. If a release has no compatible
package, use the source-build path.

## Source prerequisites

- Rust **1.88+**
- Node.js **22+**
- npm, supplied with Node.js
- Git
- Native Tauri 2 build dependencies for the host platform

Use the current
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for exact
Linux packages and Windows/macOS toolchains. Common platform requirements are:

| Platform | Native requirement |
| --- | --- |
| Linux | WebKitGTK 4.1 development files, GTK, build tools, OpenSSL, appindicator, and SVG libraries |
| macOS | Xcode Command Line Tools and the macOS SDK |
| Windows | Microsoft C++ Build Tools and WebView2 |

Check the toolchain:

```sh
rustc --version
cargo --version
node --version
npm --version
```

## Clone and install dependencies

```sh
git clone https://github.com/visorcraft/MongrelDB-Viewer.git
cd MongrelDB-Viewer
npm ci
```

`npm ci` installs the exact frontend dependency graph from `package-lock.json`.
Rust dependencies are resolved from `src-tauri/Cargo.lock` when Cargo first
builds the application.

## Development mode

```sh
npm run tauri dev
```

This starts Vite on fixed port `1420`, builds the Rust shell, and opens the
desktop window. Vite fails instead of choosing another port when `1420` is
busy.

`npm run dev` starts only the browser frontend. Most screens call Tauri
commands, so a normal browser tab is not a functional replacement for
`npm run tauri dev`.

Set a database root for one-shot development or screenshot auto-open:

```sh
VITE_AUTO_OPEN_DB=/absolute/path/to/root npm run tauri dev
```

This variable is a development helper, not a packaged application setting.

## Release build

```sh
npm run tauri build
```

The command performs the TypeScript/Vite build, compiles optimized Rust, and
asks Tauri to create the native bundles available on the host. Find results
under:

```text
src-tauri/target/release/
src-tauri/target/release/bundle/
```

Signing, notarization, package publication, and OS trust prompts depend on the
release environment. This repository does not contain private signing
credentials.

To compile the binary without producing an installer:

```sh
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --release
```

The binary is then:

```text
src-tauri/target/release/mongreldb-viewer
```

Windows adds the usual `.exe` suffix.

## Local embedding model

The default Cargo feature `local-embeddings` includes `fastembed`. ANN
installation and semantic search lazily load:

```text
all-MiniLM-L6-v2
384 dimensions
provider id: viewer-minilm
model version: 1
```

The first ANN use may download model files. Cache locations follow the OS cache
directory:

| Platform | Typical location |
| --- | --- |
| Linux | `$XDG_CACHE_HOME/mongreldb-viewer/models` or `~/.cache/mongreldb-viewer/models` |
| macOS | `~/Library/Caches/mongreldb-viewer/models` |
| Windows | `%LOCALAPPDATA%\\mongreldb-viewer\\models` |

The Rust backend has an internal remote-embedding provider, but the current
desktop UI has no remote-embedding settings form. Keep `local-embeddings`
enabled for a desktop build that needs ANN install or text-query embedding.

## Linux startup behavior

Before GTK starts, the application sets these defaults only when the caller has
not already set them:

```sh
WEBKIT_DISABLE_DMABUF_RENDERER=1
WEBKIT_DISABLE_COMPOSITING_MODE=1
```

They avoid common WebKitGTK Wayland and hybrid-GPU failures. Explicit
environment values win.

At GUI startup, the app also makes a best-effort installation of desktop files
and icons below `~/.local/share/applications` and
`~/.local/share/icons/hicolor`. Use `scripts/install-icons-linux.sh` when
developing if the compositor still shows a generic taskbar icon.

## Verify the installation

1. Launch the app.
2. Select **Direct folder**.
3. Choose an empty directory.
4. Click **Create demo DB**.
5. Confirm Deck lists six tables.
6. Run this on SQL:

   ```sql
   SELECT count(*) AS documents FROM documents;
   ```

   The result should be `8`.

For a code checkout, run the full developer gate in
[CONTRIBUTING.md](../CONTRIBUTING.md).

## Upgrade safely

1. Stop Agent or MCP activity.
2. Stop the in-app MCP server.
3. Disconnect the database.
4. Back up important roots using the engine's supported backup procedure.
5. Install the new Viewer build.
6. Open a non-production copy first when crossing engine trains.

The Viewer locks its `mongreldb-*` crates to one aligned train. A committed
older demo fixture tests one cross-version open path, but that test is not a
blanket compatibility guarantee for every historical root.

## Uninstall and retained data

Removing the application package does not delete MongrelDB roots chosen by the
user. It may leave:

- the MiniLM model cache,
- `mongreldb-viewer/flags.json` in the OS config directory,
- WebView storage containing recents and non-secret Agent configuration,
- Linux desktop/icon files installed below `~/.local/share`.

Review exact ownership before deleting any retained data. See
[operations](operations.md) for the keys and paths.

Related: [First launch](onboarding.md) · [Connections](connections.md) ·
[Troubleshooting](operations.md#troubleshooting)
