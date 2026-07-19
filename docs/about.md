# About, Licenses, and Credits

**About** (under **Sync** on the rail) is available even when no database is
open.

## About page

- Product hero (version, dual license, platform, engine metadata)  
- Feature highlights for Deck / SQL / ANN / Agent+MCP  
- Link to the public GitHub repository only  
- Buttons for **Licenses** and **Credits**  

## Licenses

In-app license viewer with tabs:

| Tab | Contents |
| --- | -------- |
| Viewer license | MIT and Apache-2.0 dual license text |
| Third-party (Rust) | cargo-about bundle of every Rust crate license |
| Frontend (npm) | Installed JavaScript packages with full license texts |
| Acknowledgments | Narrative attribution for direct deps and runtimes |
| Runtime components | WebView / GTK / related runtime licenses |

Use the find box, wrap toggle, clear, and copy controls to navigate long texts
without leaving the app.

Regenerate third-party inventories after dependency changes:

```bash
scripts/regen-credits.sh
```

This refreshes:

- `src-tauri/legal/crates.json`
- `src-tauri/legal/third-party.md`
- `src-tauri/legal/npm-packages.json`
- `src-tauri/legal/npm-third-party.md`

## Credits

- **Runtime components** - system libraries the shell links against, with
  project links and optional full license dialog  
- **npm packages** - filterable table of installed packages (name, version,
  runtime/dev role, license expression, repository)  
- **Cargo crates** - filterable table of name, version, license expression, and
  repository links  

## Policy files in the repo

- [SECURITY.md](../SECURITY.md) - private vulnerability reporting  
- [CONTRIBUTING.md](../CONTRIBUTING.md) - contribution workflow  

Related: [Onboarding](onboarding.md) · [README](../README.md)
