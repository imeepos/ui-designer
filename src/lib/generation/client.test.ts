import OpenAI from "openai";
import { describe, expect, it } from "vitest";

import { createImageClient, type InvokeFn } from "@/lib/generation/client";
import { ApiError, isApiError } from "@/lib/api/types";

function invokeWith(responses: Record<string, unknown>): InvokeFn {
  return async (command) => {
    if (command in responses) return responses[command] as never;
    throw new Error(`unexpected command ${command}`);
  };
}

describe("createImageClient", () => {
  it("builds the SDK client from the keychain key and desktop config", async () => {
    const { client, config } = await createImageClient(
      invokeWith({
        get_cms_api_key: "sk-cms-test-dummy",
        get_generation_config: { baseUrl: "https://veren.top/api", model: "gpt-image-2" },
      }),
    );
    expect(client).toBeInstanceOf(OpenAI);
    // baseURL = <serverUrl>/v1 (the adjudicated factory shape).
    expect(client.baseURL).toBe("https://veren.top/api/v1");
    expect(client.maxRetries).toBe(3);
    expect(client.timeout).toBe(300_000);
    expect(config).toEqual({ baseUrl: "https://veren.top/api", model: "gpt-image-2" });
  });

  it("guides to the account section instead of a raw error when no key exists", async () => {
    const error = await createImageClient(invokeWith({ get_cms_api_key: null })).catch(
      (e: unknown) => e,
    );
    expect(isApiError(error)).toBe(true);
    expect(error).toMatchObject({ code: "NO_CREDENTIALS" });
    expect((error as ApiError).hint).toMatch(/account section/i);
  });

  it("lets keychain failures propagate for the service boundary to map", async () => {
    // The factory never swallows Rust error envelopes: the raw {code,…}
    // rejection reaches runImageBatch, whose mapSdkError/toApiError turns it
    // into the typed KEYCHAIN_ACCESS ApiError (see service.test.ts).
    const raw = { code: "KEYCHAIN_ACCESS", message: "keychain is locked", hint: "unlock" };
    const error = await createImageClient(
      async (command) => {
        if (command === "get_cms_api_key") throw raw;
        throw new Error("unexpected");
      },
    ).catch((e: unknown) => e);
    expect(error).toBe(raw);
  });
});
