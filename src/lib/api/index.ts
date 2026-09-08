import { MockApi } from "@/lib/api/mock-api";
import { TauriApi } from "@/lib/api/tauri-api";
import type { ApiAdapter } from "@/lib/api/types";

export function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

/**
 * Mock-first: the Tauri adapter is a stub until the core bridge lands, so the
 * app always runs against MockApi unless a real Tauri shell is detected.
 */
export function createApi(): ApiAdapter {
  return hasTauriRuntime() ? new TauriApi() : new MockApi();
}

export { MockApi } from "@/lib/api/mock-api";
export { TauriApi } from "@/lib/api/tauri-api";
export * from "@/lib/api/types";
