import OpenAI from "openai";
import { describe, expect, it, vi } from "vitest";

import { ApiError, isApiError } from "@/lib/api/types";
import type { GeneratePayloadDto, InvokeFn, ProjectDetailDto } from "@/lib/api/tauri-api";
import { createImageClient, type ImageClient } from "@/lib/generation/client";
import { mapSdkError, runImageBatch, type ImageBatchParams } from "@/lib/generation/service";

// ---------------------------------------------------------------------------
// Doubles
// ---------------------------------------------------------------------------

function projectDto(): ProjectDetailDto {
  return {
    id: "p1",
    name: "Demo",
    size: { w: 1536, h: 1024, preset: "web" },
    brandBrief: "brand",
    styleBrief: "style",
    anchor: null,
    boardCandidates: [],
    pages: [],
    components: [],
    createdAt: 1,
  };
}

function payloadDto(): GeneratePayloadDto {
  return {
    candidates: [{ id: "0009", path: "/tmp/0009.png", createdAt: 99, seed: 5 }],
    project: projectDto(),
  };
}

function toUrl(path: string): string {
  return `asset://localhost/${encodeURIComponent(path)}`;
}

/** openai client double capturing images.* plus canned replies. */
function imagesDouble() {
  const calls: { generate: unknown[][]; edit: unknown[][] } = { generate: [], edit: [] };
  const client = {
    images: {
      generate: async (...args: unknown[]) => {
        calls.generate.push(args);
        return { created: 1, data: [{ b64_json: "QQ==" }] };
      },
      edit: async (...args: unknown[]) => {
        calls.edit.push(args);
        return { created: 1, data: [{ b64_json: "Qg==" }] };
      },
    },
  };
  return { calls, client: client as unknown as OpenAI };
}

function factoryFor(client: OpenAI): (invokeFn: InvokeFn) => Promise<ImageClient> {
  return async () => ({ client, config: { baseUrl: "https://veren.top/api", model: "gpt-image-2" } });
}

function makeInvoke(responses: Record<string, unknown>) {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invokeFn: InvokeFn = async (command, args) => {
    calls.push({ command, args });
    if (command in responses) return responses[command] as never;
    throw new Error(`unexpected command ${command}`);
  };
  return { calls, invokeFn };
}

function batchParams(overrides: Partial<ImageBatchParams> = {}): ImageBatchParams {
  return {
    projectId: "p1",
    kind: "board",
    target: "",
    endpoint: "generations",
    prompt: "prompt text",
    count: 2,
    size: "1536x1024",
    quality: "low",
    references: [],
    ...overrides,
  };
}

const deps = (client: OpenAI, invokeFn: InvokeFn) => ({
  invokeFn,
  toUrl,
  clientFactory: factoryFor(client),
});

// ---------------------------------------------------------------------------
// Dual-form response → record → payload refresh
// ---------------------------------------------------------------------------

describe("runImageBatch — success (payload refreshes the canvas)", () => {
  it("splits b64_json → imageBase64 and url → imageUrl, one record per item", async () => {
    const images = imagesDouble();
    // Upstream dual forms: presigned S3 URL first, b64 second, a third image.
    vi.spyOn(images.client.images, "generate").mockResolvedValue({
      created: 1,
      data: [
        { url: "https://s3.example.com/x.png?X-Amz-Signature=REDACTED" },
        { b64_json: "Qw==" },
      ],
    } as never);
    const { calls, invokeFn } = makeInvoke({ record_generated_image: payloadDto() });

    const result = await runImageBatch(batchParams(), deps(images.client, invokeFn));

    // Per item: exactly one record call, mutually exclusive source fields.
    expect(calls).toHaveLength(2);
    expect(calls[0].args).toMatchObject({
      input: {
        projectId: "p1",
        kind: "board",
        target: "",
        endpoint: "generations",
        prompt: "prompt text",
        imageUrl: "https://s3.example.com/x.png?X-Amz-Signature=REDACTED",
      },
    });
    expect(calls[0].args).not.toHaveProperty("input.imageBase64");
    expect(calls[1].args).toMatchObject({
      input: { imageBase64: "Qw==" },
    });
    expect(calls[1].args).not.toHaveProperty("input.imageUrl");

    // The presigned URL is NEVER fetched in the webview (S3 has no CORS).
    // (No global fetch stub installed: any fetch attempt would reject.)

    // The returned payload refreshes the canvas.
    expect(result.candidates[0].url).toContain("0009.png");
    expect(result.project.id).toBe("p1");
  });

  it("sends n/size/quality/model from the desktop config default", async () => {
    const images = imagesDouble();
    const { invokeFn } = makeInvoke({ record_generated_image: payloadDto() });
    await runImageBatch(batchParams({ count: 3 }), deps(images.client, invokeFn));
    const [body] = images.calls.generate[0] as [Record<string, unknown>];
    expect(body).toMatchObject({
      model: "gpt-image-2",
      prompt: "prompt text",
      n: 3,
      size: "1536x1024",
      quality: "low",
    });
  });
});

// ---------------------------------------------------------------------------
// Edits flow (multi reference images, anchor first)
// ---------------------------------------------------------------------------

describe("runImageBatch — edits with reference images", () => {
  it("passes the reference array verbatim (anchor first) to images.edit", async () => {
    const images = imagesDouble();
    const anchor = new File([new Uint8Array([1])], "anchor.png", { type: "image/png" });
    const extra = new File([new Uint8Array([2])], "extra.png", { type: "image/png" });
    const { calls, invokeFn } = makeInvoke({ record_generated_image: payloadDto() });

    await runImageBatch(
      batchParams({
        kind: "page",
        target: "dashboard",
        endpoint: "edits",
        references: [anchor, extra],
      }),
      deps(images.client, invokeFn),
    );

    expect(images.calls.generate).toHaveLength(0);
    expect(images.calls.edit).toHaveLength(1);
    const [body] = images.calls.edit[0] as [Record<string, unknown>];
    const refs = body.image as File[];
    expect(refs).toHaveLength(2);
    expect(refs[0].name).toBe("anchor.png");
    expect(refs[1].name).toBe("extra.png");
    expect(calls[0].args).toMatchObject({
      input: { kind: "page", target: "dashboard", endpoint: "edits", imageBase64: "Qg==" },
    });
  });

  it("rejects edits without the anchor", async () => {
    const images = imagesDouble();
    const { invokeFn } = makeInvoke({});
    const error = await runImageBatch(
      batchParams({ endpoint: "edits" }),
      deps(images.client, invokeFn),
    ).catch((e: unknown) => e);
    expect(error).toMatchObject({ code: "ANCHOR_REQUIRED" });
    expect(images.calls.edit).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Failure states
// ---------------------------------------------------------------------------

describe("runImageBatch — failure mapping", () => {
  it("maps a 402/quota rejection to the dedicated QUOTA_EXCEEDED code", async () => {
    const images = imagesDouble();
    vi.spyOn(images.client.images, "generate").mockRejectedValue(
      new OpenAI.APIError(
        402,
        { message: "积分不足", code: "insufficient_quota", type: "insufficient_quota" },
        "insufficient quota",
        new Headers({ "x-request-id": "r1" }),
      ),
    );
    const { invokeFn } = makeInvoke({});
    const error = await runImageBatch(batchParams(), deps(images.client, invokeFn)).catch(
      (e: unknown) => e,
    );
    expect(error).toBeInstanceOf(ApiError);
    expect(error).toMatchObject({ code: "QUOTA_EXCEEDED", message: "积分不足" });
  });

  it("maps network failures to API_ERROR with a reachability hint", async () => {
    const images = imagesDouble();
    vi.spyOn(images.client.images, "generate").mockRejectedValue(
      new OpenAI.APIConnectionError({ message: "fetch failed" }),
    );
    const { invokeFn } = makeInvoke({});
    const error = await runImageBatch(batchParams(), deps(images.client, invokeFn)).catch(
      (e: unknown) => e,
    );
    expect(error).toMatchObject({
      code: "API_ERROR",
      hint: "the gateway is unreachable — check the network",
    });
  });

  it("rejects before any record call when no key exists (guide to sign-in)", async () => {
    const { calls, invokeFn } = makeInvoke({ get_cms_api_key: null });
    const error = await runImageBatch(batchParams(), {
      invokeFn,
      toUrl,
      clientFactory: createImageClient,
    }).catch((e: unknown) => e);
    expect(error).toMatchObject({ code: "NO_CREDENTIALS" });
    expect(calls.every((call) => call.command !== "record_generated_image")).toBe(true);
  });

  it("reports an empty upstream batch instead of persisting nothing silently", async () => {
    const images = imagesDouble();
    vi.spyOn(images.client.images, "generate").mockResolvedValue({
      created: 1,
      data: [],
    } as never);
    const { calls, invokeFn } = makeInvoke({ record_generated_image: payloadDto() });
    const error = await runImageBatch(batchParams(), deps(images.client, invokeFn)).catch(
      (e: unknown) => e,
    );
    expect(error).toMatchObject({ code: "API_ERROR" });
    expect(calls).toHaveLength(0);
  });
});

describe("mapSdkError", () => {
  it("keeps typed ApiErrors untouched", () => {
    const original = new ApiError("CANCELLED", "cancelled");
    expect(mapSdkError(original)).toBe(original);
  });

  it("maps a rejected key (401) back to sign-in guidance", () => {
    const error = mapSdkError(
      new OpenAI.APIError(
        401,
        { message: "invalid api key" },
        "invalid key",
        new Headers(),
      ),
    );
    expect(isApiError(error)).toBe(true);
    expect(error).toMatchObject({ code: "NO_CREDENTIALS" });
  });
});
