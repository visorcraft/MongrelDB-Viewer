/** True when running on macOS (or iOS). */
export function isApplePlatform(): boolean {
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent || "";
  const platform = navigator.platform || "";
  // userAgentData is Chromium-only; fall back to classic signals.
  const uaData = (navigator as Navigator & { userAgentData?: { platform?: string } })
    .userAgentData?.platform;
  const hay = `${ua} ${platform} ${uaData ?? ""}`;
  return /Mac|iPhone|iPad|iPod|Macintosh/i.test(hay);
}

/** Mod+F shortcut label for the command palette. */
export function paletteShortcutLabel(): string {
  return isApplePlatform() ? "⌘F" : "Ctrl+F";
}

/** Help text pieces: modifier key name + key. */
export function paletteShortcutParts(): { mod: string; key: string } {
  return isApplePlatform()
    ? { mod: "⌘", key: "F" }
    : { mod: "Ctrl", key: "F" };
}
