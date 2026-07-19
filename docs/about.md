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
| Third-party | cargo-about bundle of Rust crate licenses |
| Acknowledgments | Narrative attribution |
| Runtime components | WebView / GTK / related runtime licenses |

Use the find box, wrap toggle, clear, and copy controls to navigate long texts
without leaving the app.

Regenerate third-party data after dependency changes:

```bash
scripts/regen-credits.sh
```

## Credits

- **Runtime components** - system libraries the shell links against, with
  project links and optional full license dialog  
- **Cargo crates** - filterable table of name, version, license expression, and
  repository links  

## Policy files in the repo

- [SECURITY.md](../SECURITY.md) - private vulnerability reporting  
- [CONTRIBUTING.md](../CONTRIBUTING.md) - contribution workflow  

Related: [Onboarding](onboarding.md) · [README](README.md)
