#!/usr/bin/env bash
#
# install-icons-linux.sh - install MongrelDB Viewer's helmet icon +
# .desktop entries so the Wayland / X11 taskbar shows the helmet when
# running the binary directly (tauri dev / target/debug / target/release).
#
# Background: on Wayland the compositor does not use window icons set by
# the app. It matches the window's app_id against an installed
# `${app_id}.desktop` and loads Icon= from the hicolor theme.
#
# With enableGTKAppId=true the app_id is the Tauri identifier
# (com.visorcraft.mongreldb-viewer). We also install a basename entry
# (mongreldb-viewer.desktop) for builds that leave gtk app id off.
#
# Usage:
#   scripts/install-icons-linux.sh
#   scripts/install-icons-linux.sh --uninstall

set -euo pipefail

cd "$(dirname "$0")/.."

PRIMARY_ID="com.visorcraft.mongreldb-viewer"
FALLBACK_ID="mongreldb-viewer"
DISPLAY_NAME="MongrelDB Viewer"

ICON_DIR="$HOME/.local/share/icons/hicolor"
APPS_DIR="$HOME/.local/share/applications"

SRC_ICONS=(
  "src-tauri/icons/32x32.png:32x32"
  "src-tauri/icons/64x64.png:64x64"
  "src-tauri/icons/128x128.png:128x128"
  "src-tauri/icons/128x128@2x.png:256x256"
  "src-tauri/icons/icon.png:512x512"
)

# Prefer release binary, then debug, then PATH.
resolve_exec() {
  if [[ -x "$(pwd)/src-tauri/target/release/mongreldb-viewer" ]]; then
    echo "$(pwd)/src-tauri/target/release/mongreldb-viewer"
  elif [[ -x "$(pwd)/src-tauri/target/debug/mongreldb-viewer" ]]; then
    echo "$(pwd)/src-tauri/target/debug/mongreldb-viewer"
  elif command -v mongreldb-viewer >/dev/null 2>&1; then
    command -v mongreldb-viewer
  else
    echo "$(pwd)/src-tauri/target/debug/mongreldb-viewer"
  fi
}

write_desktop() {
  local app_id="$1"
  local exec_path="$2"
  local desktop_file="$APPS_DIR/${app_id}.desktop"
  cat > "$desktop_file" <<EOF
[Desktop Entry]
Type=Application
Name=${DISPLAY_NAME}
GenericName=Database Viewer
Comment=Signal Deck for AI-native MongrelDB databases
Exec=${exec_path} %U
TryExec=${exec_path}
Icon=${PRIMARY_ID}
Terminal=false
StartupNotify=true
StartupWMClass=${app_id}
Categories=Development;Database;
Keywords=mongreldb;ann;hnsw;sql;vector;embedding;
EOF
  chmod 0644 "$desktop_file"
  echo "  entry        ->  $desktop_file"
}

uninstall() {
  echo "Removing MongrelDB Viewer icon + .desktop entries from ~/.local/share/"
  for entry in "${SRC_ICONS[@]}"; do
    size="${entry##*:}"
    rm -f "$ICON_DIR/$size/apps/${PRIMARY_ID}.png"
    rm -f "$ICON_DIR/$size/apps/${FALLBACK_ID}.png"
  done
  rm -f "$APPS_DIR/${PRIMARY_ID}.desktop" "$APPS_DIR/${FALLBACK_ID}.desktop"
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPS_DIR" 2>/dev/null || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -t -f "$ICON_DIR" 2>/dev/null || true
  fi
  echo "Done."
}

action="${1:-install}"
if [[ "$action" == "--uninstall" || "$action" == "-u" ]]; then
  uninstall
  exit 0
fi

echo "Installing MongrelDB Viewer helmet icon + .desktop entries to ~/.local/share/"

for entry in "${SRC_ICONS[@]}"; do
  src="${entry%%:*}"
  size="${entry##*:}"
  if [[ ! -f "$src" ]]; then
    echo "  skip missing $src" >&2
    continue
  fi
  dest_dir="$ICON_DIR/$size/apps"
  mkdir -p "$dest_dir"
  cp -f "$src" "$dest_dir/${PRIMARY_ID}.png"
  # Alias so either app_id resolves the same helmet art.
  cp -f "$src" "$dest_dir/${FALLBACK_ID}.png"
  echo "  icon  $size  ->  $dest_dir/${PRIMARY_ID}.png"
done

mkdir -p "$APPS_DIR"
EXEC_PATH="$(resolve_exec)"
write_desktop "$PRIMARY_ID" "$EXEC_PATH"
write_desktop "$FALLBACK_ID" "$EXEC_PATH"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" 2>/dev/null || true
  echo "  refreshed update-desktop-database"
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -t -f "$ICON_DIR" 2>/dev/null || true
  echo "  refreshed gtk-update-icon-cache"
fi
if [[ "${XDG_CURRENT_DESKTOP:-}" == *"KDE"* ]] && command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 2>/dev/null || true
  echo "  refreshed KDE sycoca"
elif [[ "${XDG_CURRENT_DESKTOP:-}" == *"KDE"* ]] && command -v kbuildsycoca5 >/dev/null 2>&1; then
  kbuildsycoca5 2>/dev/null || true
  echo "  refreshed KDE sycoca (Plasma 5)"
fi

echo ""
echo "Installed. Restart MongrelDB Viewer - the taskbar should show the helmet."
echo "Undo with: scripts/install-icons-linux.sh --uninstall"
