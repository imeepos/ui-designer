import { invoke } from "@tauri-apps/api/core";

import { hasTauriRuntime } from "@/lib/api";
import { toApiError } from "@/lib/api/tauri-api";
import { ApiError } from "@/lib/api/types";

/**
 * Shell-level credential API (Settings dialog): reads/writes the OS keychain
 * through dedicated Tauri commands. Keys are NEVER returned in full — status
 * carries only the source label (`env`/`keychain`/`none`) and the masked
 * tail (last 4 chars). In the browser mock preview these calls degrade
 * gracefully: status reads a neutral stub, mutations reject NOT_IMPLEMENTED.
 */

export type CredentialKeySource = "env" | "keychain" | "none";

export interface CredentialStatus {
  /** Effective base URL after the env → config → default chain. */
  baseUrl: string;
  /** Effective model name after the OPENAI_MODEL → config → default chain. */
  model: string;
  keySource: CredentialKeySource;
  /** Last 4 characters of the resolved key, when one is configured. */
  keyTail: string | null;
}

export interface ConnectionTestResult {
  baseUrl: string;
  httpStatus: number;
  modelCount: number;
  /** The effective model probed (env → config → default). */
  model: string;
  /** All visible model ids (settings dialog datalist). */
  models: string[];
}

export interface TestConnectionDraft {
  /** Draft from the input; empty falls back to the stored/resolved chain. */
  baseUrl?: string;
  /** Draft key from the input; empty falls back to the stored/resolved key. */
  apiKey?: string;
}

export interface SaveConfigDraft {
  baseUrl: string;
  /** Model name to store; undefined leaves the stored value untouched. */
  model?: string;
}

const MOCK_STATUS: CredentialStatus = {
  baseUrl: "",
  model: "gpt-image-2",
  keySource: "none",
  keyTail: null,
};

function requireDesktop(): void {
  if (!hasTauriRuntime()) {
    throw new ApiError(
      "NOT_IMPLEMENTED",
      "credential storage needs the desktop shell",
      "launch the Tauri app to manage the API key",
    );
  }
}

export async function getCredentialStatus(): Promise<CredentialStatus> {
  if (!hasTauriRuntime()) return MOCK_STATUS;
  try {
    return await invoke<CredentialStatus>("get_credential_status");
  } catch (error) {
    throw toApiError(error);
  }
}

export async function saveApiKey(apiKey: string): Promise<CredentialStatus> {
  requireDesktop();
  try {
    return await invoke<CredentialStatus>("save_api_key", { input: { apiKey } });
  } catch (error) {
    throw toApiError(error);
  }
}

export async function clearApiKey(): Promise<CredentialStatus> {
  requireDesktop();
  try {
    return await invoke<CredentialStatus>("clear_api_key");
  } catch (error) {
    throw toApiError(error);
  }
}

/**
 * Persist the non-sensitive fields (base URL + model name) into
 * ~/Rudder/config.json. The API key NEVER goes through here — it lives in
 * the OS keychain only. An empty baseUrl / model resets to the default chain.
 */
export async function saveConfig(draft: SaveConfigDraft): Promise<CredentialStatus> {
  requireDesktop();
  try {
    return await invoke<CredentialStatus>("save_base_url", {
      baseUrl: draft.baseUrl,
      model: draft.model,
    });
  } catch (error) {
    throw toApiError(error);
  }
}

export async function testConnection(
  draft: TestConnectionDraft = {},
): Promise<ConnectionTestResult> {
  requireDesktop();
  try {
    return await invoke<ConnectionTestResult>("test_connection", { input: draft });
  } catch (error) {
    throw toApiError(error);
  }
}
