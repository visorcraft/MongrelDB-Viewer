//! Linux display / WebKitGTK startup hardening + Wayland taskbar icon.
//!
//! WebKitGTK on Wayland commonly dies with:
//!   `Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.`
//! when DMA-BUF rendering is negotiated against a compositor that rejects the
//! buffer. Setting these before any GTK/WebKit init keeps native Wayland and
//! avoids forcing `GDK_BACKEND=x11`.
//!
//! Existing environment values always win so operators can override.
//!
//! Wayland taskbars ignore window icons set by the app. They match the
//! window `app_id` (GTK application id / binary basename) against a
//! FreeDesktop `.desktop` file and load `Icon=` from the hicolor theme.
//! [`ensure_desktop_integration`] installs those files under ~/.local/share.

/// Apply safe defaults for Linux GUI startup. No-op on non-Linux.
pub fn apply_web_display_defaults() {
    #[cfg(target_os = "linux")]
    {
        // Primary fix for Wayland protocol error 71 with WebKitGTK / WRY.
        set_default("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        // Avoids a class of offscreen compositing crashes on hybrid GPU setups.
        set_default("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        // Prefer a stable GTK renderer when the session has not chosen one.
        // `ngl` is the modern default; if unset and Wayland is active, leave
        // GSK alone unless the user is on a known-broken path - dmabuf is enough.
        let _ = std::env::var_os("WAYLAND_DISPLAY");
    }
}

/// Install helmet icons + .desktop entries for Wayland/X11 taskbar (best-effort).
pub fn ensure_desktop_integration() {
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = install_desktop_integration() {
            tracing::warn!("desktop icon integration: {e}");
        }
    }
}

#[cfg(target_os = "linux")]
fn set_default(key: &str, value: &str) {
    if std::env::var_os(key).is_none() {
        // SAFETY: called once at process start, before other threads spawn GUI work.
        unsafe {
            std::env::set_var(key, value);
        }
    }
}

#[cfg(target_os = "linux")]
fn install_desktop_integration() -> Result<(), String> {
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    // Matches tauri.conf.json identifier with enableGTKAppId=true.
    const PRIMARY_ID: &str = "com.visorcraft.mongreldb-viewer";
    // Fallback when gtk app id is off (binary basename).
    const FALLBACK_ID: &str = "mongreldb-viewer";
    const DISPLAY_NAME: &str = "MongrelDB Viewer";

    let home = dirs::home_dir().ok_or_else(|| "no home dir".to_string())?;
    let icon_root = home.join(".local/share/icons/hicolor");
    let apps_dir = home.join(".local/share/applications");
    fs::create_dir_all(&apps_dir).map_err(|e| e.to_string())?;

    // Embedded helmet assets (regenerated via `tauri icon`).
    let icons: &[(&str, &[u8])] = &[
        ("32x32", include_bytes!("../icons/32x32.png")),
        ("64x64", include_bytes!("../icons/64x64.png")),
        ("128x128", include_bytes!("../icons/128x128.png")),
        ("256x256", include_bytes!("../icons/128x128@2x.png")),
        ("512x512", include_bytes!("../icons/icon.png")),
    ];

    for (size, bytes) in icons {
        let dir = icon_root.join(size).join("apps");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for id in [PRIMARY_ID, FALLBACK_ID] {
            let path = dir.join(format!("{id}.png"));
            // Skip rewrite if identical size (cheap check) and file exists.
            if path.is_file()
                && fs::metadata(&path).map(|m| m.len()).unwrap_or(0) == bytes.len() as u64
            {
                continue;
            }
            fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
        }
    }

    let exec = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .unwrap_or_else(|_| {
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mongreldb-viewer"))
        });
    let exec_s = exec.display().to_string();

    for app_id in [PRIMARY_ID, FALLBACK_ID] {
        let desktop = apps_dir.join(format!("{app_id}.desktop"));
        let body = format!(
            "\
[Desktop Entry]
Type=Application
Name={DISPLAY_NAME}
GenericName=Database Viewer
Comment=Signal Deck for AI-native MongrelDB databases
Exec={exec_s} %U
TryExec={exec_s}
Icon={PRIMARY_ID}
Terminal=false
StartupNotify=true
StartupWMClass={app_id}
Categories=Development;Database;
Keywords=mongreldb;ann;hnsw;sql;vector;embedding;
"
        );
        let mut f = fs::File::create(&desktop).map_err(|e| e.to_string())?;
        f.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    }

    // Best-effort cache refresh (compositors often still need an app restart).
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .status();
    let _ = std::process::Command::new("gtk-update-icon-cache")
        .args(["-t", "-f"])
        .arg(&icon_root)
        .status();
    if std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.contains("KDE"))
        .unwrap_or(false)
    {
        let _ = std::process::Command::new("kbuildsycoca6").status();
        let _ = std::process::Command::new("kbuildsycoca5").status();
    }

    Ok(())
}
