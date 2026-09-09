/**
 * Shell-level CustomEvent bus: lets a global toast (outside React dialog
 * state) ask the app shell to open the Settings dialog.
 */
export const OPEN_SETTINGS_EVENT = "rudder:open-settings";

export function requestOpenSettings(): void {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(OPEN_SETTINGS_EVENT));
  }
}
