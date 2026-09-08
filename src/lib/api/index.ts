import { MockApi } from "@/lib/api/mock-api";
import { TauriApi } from "@/lib/api/tauri-api";
import type { ApiAdapter } from "@/lib/api/types";

/**
 * Tauri 2 injects `__TAURI_INTERNALS__` into the webview; `__TAURI__` only
 * exists with `withGlobalTauri`. Browser dev (vite/vitest) has neither and
 * falls back to MockApi.
 */
export function hasTauriRuntime(): boolean {
  if (typeof window === "undefined") return false;
  return "__TAURI_INTERNALS__" in window || "__TAURI__" in window;
}

/**
 * Mock-first: the Tauri adapter talks to the real Rust bridge (rudder-core)
 * and is only used inside the desktop shell; plain browser dev uses MockApi.
 */
export function createApi(): ApiAdapter {
  return hasTauriRuntime() ? new TauriApi() : new MockApi();
}

export { MockApi } from "@/lib/api/mock-api";
export { TauriApi } from "@/lib/api/tauri-api";
export * from "@/lib/api/types";
