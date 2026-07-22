//! Bundled license and credits data for the in-app About / Licenses / Credits views.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    pub app_name: String,
    pub version: String,
    pub license: String,
    pub repository: String,
    pub git_sha: String,
    pub engine_version: String,
    pub query_version: String,
    pub description: String,
    pub tagline: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrateCredit {
    pub name: String,
    pub version: String,
    pub license: String,
    pub repository: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpmPackageCredit {
    pub name: String,
    pub version: String,
    pub license: String,
    pub repository: String,
    /// `"runtime"` for production dependencies, `"dev"` for build tooling.
    pub role: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeComponent {
    pub name: String,
    pub licenses: String,
    pub spdx: Vec<String>,
    pub project_url: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditsData {
    pub crates: Vec<CrateCredit>,
    pub packages: Vec<NpmPackageCredit>,
    pub runtime: Vec<RuntimeComponent>,
    pub crate_count: usize,
    pub package_count: usize,
    pub runtime_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseDocMeta {
    pub id: String,
    pub title: String,
    pub subtitle: String,
}

pub fn about_info() -> AboutInfo {
    let engine = mongreldb_core::build_info();
    let query = mongreldb_query::build_info();
    AboutInfo {
        app_name: "MongrelDB Viewer".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        license: "MIT OR Apache-2.0".into(),
        repository: "https://github.com/visorcraft/MongrelDB-Viewer".into(),
        git_sha: engine.mongreldb_git_sha.chars().take(7).collect(),
        engine_version: engine.engine_version.to_string(),
        query_version: query.query_version.to_string(),
        description: "Signal Deck for AI-native MongrelDB databases.".into(),
        tagline: "Built on Rust + Tauri 2 + React.".into(),
        platform: std::env::consts::OS.into(),
    }
}

pub fn license_docs() -> Vec<LicenseDocMeta> {
    vec![
        LicenseDocMeta {
            id: "app".into(),
            title: "Viewer license".into(),
            subtitle: "MIT and Apache-2.0 dual license text bundled into the application.".into(),
        },
        LicenseDocMeta {
            id: "third-party".into(),
            title: "Third-party (Rust)".into(),
            subtitle: "cargo-about bundle with every direct and transitive Rust crate, grouped by license text.".into(),
        },
        LicenseDocMeta {
            id: "npm".into(),
            title: "Frontend (npm)".into(),
            subtitle: "Installed JavaScript packages (runtime and build tooling) with full license texts.".into(),
        },
        LicenseDocMeta {
            id: "acknowledgments".into(),
            title: "Acknowledgments".into(),
            subtitle: "Narrative attribution for MongrelDB Viewer, runtime components, and direct dependencies.".into(),
        },
        LicenseDocMeta {
            id: "runtime".into(),
            title: "Runtime components".into(),
            subtitle: "Full license texts for WebKitGTK, GTK, GLib, and related runtimes the shell builds on.".into(),
        },
    ]
}

pub fn license_document(id: &str) -> AppResult<String> {
    match id {
        "app" | "app-mit" | "mit" => Ok(format!(
            "# MongrelDB Viewer - dual license\n\n\
             MongrelDB Viewer is dual-licensed under **MIT OR Apache-2.0**.\n\
             You may choose either license.\n\n\
             ---\n\n\
             # MIT License\n\n\
             {}\n\n\
             ---\n\n\
             # Apache License 2.0\n\n\
             {}\n",
            include_str!("../legal/LICENSE-MIT.txt").trim(),
            include_str!("../legal/LICENSE-APACHE.txt").trim()
        )),
        "app-apache" | "apache" => Ok(include_str!("../legal/LICENSE-APACHE.txt").into()),
        "third-party" | "third_party" => Ok(include_str!("../legal/third-party.md").into()),
        "npm" | "frontend" | "npm-third-party" => {
            Ok(include_str!("../legal/npm-third-party.md").into())
        }
        "acknowledgments" | "credits-narrative" => {
            Ok(include_str!("../legal/acknowledgments.md").into())
        }
        "runtime" | "runtime-components" => Ok(include_str!("../legal/runtime.md").into()),
        other => Err(AppError::msg(format!("unknown license document: {other}"))),
    }
}

pub fn credits_data() -> AppResult<CreditsData> {
    let crates: Vec<CrateCredit> = serde_json::from_str(include_str!("../legal/crates.json"))
        .map_err(|e| AppError::msg(format!("failed to parse bundled crates.json: {e}")))?;
    let packages: Vec<NpmPackageCredit> =
        serde_json::from_str(include_str!("../legal/npm-packages.json")).map_err(|e| {
            AppError::msg(format!("failed to parse bundled npm-packages.json: {e}"))
        })?;
    let runtime = runtime_components();
    let crate_count = crates.len();
    let package_count = packages.len();
    let runtime_count = runtime.len();
    Ok(CreditsData {
        crates,
        packages,
        runtime,
        crate_count,
        package_count,
        runtime_count,
    })
}

pub fn runtime_components() -> Vec<RuntimeComponent> {
    vec![
        RuntimeComponent {
            name: "WebKitGTK / WRY".into(),
            licenses: "LGPL-2.1+ / BSD (mixed)".into(),
            spdx: vec!["LGPL-2.1-or-later".into()],
            project_url: "https://webkitgtk.org".into(),
            notes: "Linux WebView backend used by Tauri/WRY".into(),
        },
        RuntimeComponent {
            name: "GTK 3".into(),
            licenses: "LGPL-2.1-or-later".into(),
            spdx: vec!["LGPL-2.1-or-later".into()],
            project_url: "https://www.gtk.org".into(),
            notes: "Windowing toolkit on Linux".into(),
        },
        RuntimeComponent {
            name: "GLib / GObject".into(),
            licenses: "LGPL-2.1-or-later".into(),
            spdx: vec!["LGPL-2.1-or-later".into()],
            project_url: "https://docs.gtk.org/glib/".into(),
            notes: "Core GObject event loop primitives".into(),
        },
        RuntimeComponent {
            name: "Cairo".into(),
            licenses: "LGPL-2.1 / MPL-1.1".into(),
            spdx: vec!["LGPL-2.1-or-later".into()],
            project_url: "https://www.cairographics.org".into(),
            notes: "2D graphics library".into(),
        },
        RuntimeComponent {
            name: "libsoup".into(),
            licenses: "LGPL-2.1-or-later".into(),
            spdx: vec!["LGPL-2.1-or-later".into()],
            project_url: "https://libsoup.org".into(),
            notes: "HTTP client library used with WebKitGTK".into(),
        },
        RuntimeComponent {
            name: "OpenSSL (system, if present)".into(),
            licenses: "Apache-2.0".into(),
            spdx: vec!["Apache-2.0".into()],
            project_url: "https://www.openssl.org".into(),
            notes: "TLS when provided by the host stack".into(),
        },
        RuntimeComponent {
            name: "Microsoft Edge WebView2 (Windows)".into(),
            licenses: "Microsoft proprietary (system runtime)".into(),
            spdx: vec![],
            project_url: "https://developer.microsoft.com/microsoft-edge/webview2/".into(),
            notes: "Windows WebView host; not redistributed by this package".into(),
        },
        RuntimeComponent {
            name: "WebKit / WKWebView (macOS)".into(),
            licenses: "Apple system framework terms".into(),
            spdx: vec![],
            project_url: "https://webkit.org".into(),
            notes: "macOS WebView host".into(),
        },
    ]
}

pub fn runtime_license_text(spdx_id: &str) -> AppResult<String> {
    match spdx_id {
        "LGPL-2.1-or-later" | "LGPL-2.1" => {
            Ok(include_str!("../legal/runtime/LGPL-2.1-or-later.txt").into())
        }
        "LGPL-3.0-only" | "LGPL-3.0" => {
            Ok(include_str!("../legal/runtime/LGPL-3.0-only.txt").into())
        }
        "GPL-2.0-or-later" | "GPL-2.0" => {
            Ok(include_str!("../legal/runtime/GPL-2.0-or-later.txt").into())
        }
        "Apache-2.0" => Ok(include_str!("../legal/runtime/Apache-2.0.txt").into()),
        other => Err(AppError::msg(format!("unknown runtime SPDX id: {other}"))),
    }
}
