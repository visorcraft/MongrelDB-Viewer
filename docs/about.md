# About, licenses, and credits

**About** is available with or without a database connection. It reports build
metadata and provides offline access to application and dependency notices.

![About page](images/08-about.png)

## About metadata

The page shows:

- application name and version,
- platform,
- `MIT OR Apache-2.0`,
- repository link,
- linked MongrelDB engine and query versions,
- short MongrelDB git SHA,
- stack summary.

Engine/query values are compiled into Viewer. In Server mode they do not query
the remote daemon's version.

## License documents

| Tab | Bundled content |
| --- | --- |
| Viewer license | Full MIT and Apache-2.0 texts |
| Third-party (Rust) | `cargo-about` output for direct and transitive crates |
| Frontend (npm) | Installed runtime and build packages grouped by license text |
| Acknowledgments | Narrative attribution and direct-dependency roles |
| Runtime components | WebView, GTK, GLib, Cairo, libsoup, and related notices |

Controls support:

- find within the document,
- line wrapping,
- clearing search,
- copying document text.

Documents are compiled from the committed inventory into the application
binary and do not require network access. Regeneration after every dependency
change is what keeps that inventory aligned with the built artifact.

## Credits

Credits loads three structured inventories:

| Inventory | Fields |
| --- | --- |
| Cargo crates | name, version, license expression, repository |
| npm packages | name, version, runtime/dev role, license expression, repository |
| Runtime components | name, license family, SPDX IDs, project URL, notes |

Crate and package tables are filterable. Runtime entries with a bundled SPDX
text can open that full license.

## Repository sources

```text
LICENSE-MIT
LICENSE-APACHE
src-tauri/legal/LICENSE-MIT.txt
src-tauri/legal/LICENSE-APACHE.txt
src-tauri/legal/crates.json
src-tauri/legal/npm-packages.json
src-tauri/legal/third-party.md
src-tauri/legal/npm-third-party.md
src-tauri/legal/acknowledgments.md
src-tauri/legal/runtime.md
src-tauri/legal/runtime/
```

The top-level licenses govern MongrelDB Viewer. Third-party components remain
under their own terms.

## Regenerate after dependency changes

Install the generator prerequisite:

```sh
cargo install cargo-about
```

Ensure npm packages are installed, then run:

```sh
npm ci
scripts/regen-credits.sh
```

The script requires Bash, Python 3, Cargo, npm's installed package tree, and
`cargo-about`. It:

1. reads Cargo metadata and writes `crates.json`,
2. reads `package-lock.json` and installed package manifests,
3. finds npm license files and writes `npm-packages.json`,
4. groups npm license texts into `npm-third-party.md`,
5. updates direct-dependency versions in acknowledgments,
6. runs `cargo about generate` for Rust license text.

Review and commit all generated changes with the dependency change:

```text
src-tauri/legal/crates.json
src-tauri/legal/third-party.md
src-tauri/legal/npm-packages.json
src-tauri/legal/npm-third-party.md
src-tauri/legal/acknowledgments.md
```

Do not hand-edit generated inventories to hide a dependency. Fix metadata or
the generator input, regenerate, and review the result.

## Policy

- Contribution terms: [CONTRIBUTING.md](../CONTRIBUTING.md)
- Security reporting: [SECURITY.md](../SECURITY.md)
- Viewer licenses: [MIT](../LICENSE-MIT) and
  [Apache-2.0](../LICENSE-APACHE)

Related: [Architecture](architecture.md#legal-document-pipeline) ·
[Installation](installation.md)
