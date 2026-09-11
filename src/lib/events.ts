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

/**
 * Tree "+" entries ask the detail panel to reveal the matching add form
 * (page / component) without threading UI state through the store.
 */
export const OPEN_TREE_ADD_EVENT = "rudder:open-tree-add";

export type TreeAddKind = "page" | "component";

export function requestTreeAdd(kind: TreeAddKind): void {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(OPEN_TREE_ADD_EVENT, { detail: { kind } }));
  }
}

/**
 * A finished generation deducted points: the account section re-reads
 * `auth_status` so the balance display stays current without reopening.
 */
export const BALANCE_REFRESH_EVENT = "rudder:balance-refresh";

export function requestBalanceRefresh(): void {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(BALANCE_REFRESH_EVENT));
  }
}
