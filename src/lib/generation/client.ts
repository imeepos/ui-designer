import OpenAI from "openai";

import { ApiError } from "@/lib/api/types";

/**
 * SDK client factory for the cms-direct generation path (C2-FE, route A).
 *
 * The openai SDK runs inside the Tauri webview and talks to the cms gateway
 * directly (G1 CORS probes passed); Rust stays out of the request path. The key
 * travels `get_cms_api_key` → this factory → the client instance in memory
 * lives in webview memory ONLY: never persisted, rendered or logged.
 */

export interface GenerationConfig {
  /** Server base without `/v1` (e.g. `https://veren.top/api`). */
  baseUrl: string;
  /** Default image model (e.g. `gpt-image-2`). */
  model: string;
}

export type InvokeFn = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

/** `get_generation_config` reply (commands.rs `GenerationConfigDto`). */
export interface GenerationConfigDto {
  baseUrl: string;
  model: string;
}

/** Read the desktop generation endpoint config (thin read-only mirror). */
export async function fetchGenerationConfig(
  invokeFn: InvokeFn,
): Promise<GenerationConfig> {
  const dto = await invokeFn<GenerationConfigDto>("get_generation_config");
  return { baseUrl: dto.baseUrl, model: dto.model };
}

/** Client + config bundle the generation service needs per batch. */
export interface ImageClient {
  client: OpenAI;
  config: GenerationConfig;
}

/**
 * Build the openai client against the cms gateway. Rejects
 * `NO_CREDENTIALS` when no cms key exists (signed out) so the caller can
 * guide the user to the account section instead of a raw network failure.
 *
 * `maxRetries: 3` / `timeout: 300000` mirror the Rust ImageClient semantics;
 * `dangerouslyAllowBrowser` is required for the webview runtime.
 */
export async function createImageClient(invokeFn: InvokeFn): Promise<ImageClient> {
  // Keychain failures keep their Rust error code (KEYCHAIN_ACCESS) and are
  // never logged here — the raw invoke rejection propagates untouched.
  const key = await invokeFn<string | null>("get_cms_api_key");
  if (!key) {
    throw new ApiError(
      "NO_CREDENTIALS",
      "no cms api key: sign in to mint one",
      "sign in from the account section, then retry",
    );
  }
  const config = await fetchGenerationConfig(invokeFn);
  const client = new OpenAI({
    apiKey: key,
    baseURL: `${config.baseUrl}/v1`,
    maxRetries: 3,
    timeout: 300_000,
    dangerouslyAllowBrowser: true,
  });
  return { client, config };
}
