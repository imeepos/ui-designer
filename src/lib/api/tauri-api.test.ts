import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { ImageClient } from "@/lib/generation/client";
import { ApiError } from "@/lib/api/types";
import { TauriApi, type GeneratePayloadDto, type InvokeFn, type ProjectDetailDto } from "@/lib/api/tauri-api";
import type OpenAI from "openai";

function projectDto(): ProjectDetailDto {
  return {
    id: "p1",
    name: "Demo",
    size: { w: 1536, h: 1024, preset: "web" },
    brandBrief: "brand",
    styleBrief: "style",
    anchor: {
      candidateId: "0001",
      path: "/home/u/Rudder/projects/x/board/anchor.png",
      createdAt: 100,
    },
    boardCandidates: [
      { id: "0001", path: "/home/u/Rudder/projects/x/board/candidates/0001.png", createdAt: 90, seed: 7 },
    ],
    pages: [
      {
        slug: "dashboard",
        brief: "b",
        candidates: [
          { id: "0002", path: "/home/u/Rudder/projects/x/pages/dashboard/candidates/0002.png", createdAt: 91 },
        ],
        current: { path: "/home/u/Rudder/projects/x/pages/dashboard/current.png", candidateId: "0002" },
        history: [{ ts: 80, path: "/home/u/Rudder/projects/x/pages/dashboard/history/h.png" }],
        updatedAt: 92,
      },
    ],
    components: [],
    createdAt: 1,
  };
}

/** Board variant before its anchor exists → the SDK `generations` endpoint. */
function projectDtoNoAnchor(): ProjectDetailDto {
  return { ...projectDto(), anchor: null };
}

/** Double for the openai client capturing images.* calls. */
function fakeImagesClient() {
  const calls: { generate: unknown[][]; edit: unknown[][] } = { generate: [], edit: [] };
  let generateReply: unknown = { created: 1, data: [{ b64_json: "AAAA" }] };
  let editReply: unknown = { created: 1, data: [{ b64_json: "QQ==" }] };
  const client = {
    images: {
      generate: async (...args: unknown[]) => {
        calls.generate.push(args);
        if (generateReply instanceof Error) throw generateReply;
        return generateReply;
      },
      edit: async (...args: unknown[]) => {
        calls.edit.push(args);
        if (editReply instanceof Error) throw editReply;
        return editReply;
      },
    },
  };
  return {
    calls,
    client: client as unknown as OpenAI,
    setGenerateReply: (reply: unknown) => {
      generateReply = reply;
    },
    setEditReply: (reply: unknown) => {
      editReply = reply;
    },
    pendingEdit(): { release: (value: unknown) => void } {
      let resolveEdit: (value: unknown) => void = () => {};
      client.images.edit = async (...args: unknown[]) => {
        calls.edit.push(args);
        return new Promise((resolve) => {
          resolveEdit = resolve;
        }) as never;
      };
      return {
        // Late-bound: the resolver only exists once edit() has been called.
        release: (value: unknown) => resolveEdit(value),
      };
    },
  };
}

function fakeImageClientFactory(client: OpenAI): (invokeFn: InvokeFn) => Promise<ImageClient> {
  return async () => ({
    client,
    config: { baseUrl: "https://veren.top/api", model: "gpt-image-2" },
  });
}

/** Answer the asset-protocol fetch the reference loader performs. */
function stubAnchorFetch() {
  const fetchMock = vi.fn(async () =>
    new Response(new Blob([new Uint8Array([0x89, 0x50])], { type: "image/png" }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function generateDto(): GeneratePayloadDto {
  return {
    candidates: [{ id: "0003", path: "/tmp/0003.png", createdAt: 95, seed: 8 }],
    project: projectDto(),
  };
}

function toUrl(path: string): string {
  return `asset://localhost/${encodeURIComponent(path)}`;
}

/** Invoke double: records calls, answers from a queue. */
function makeInvoke(responses: Record<string, unknown> = {}) {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invokeFn: InvokeFn = async (command, args) => {
    calls.push({ command, args });
    if (command in responses) return responses[command] as never;
    throw new Error(`unexpected command ${command}`);
  };
  return { calls, invokeFn };
}

describe("TauriApi", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("maps create_project DTO paths into asset urls", async () => {
    const { calls, invokeFn } = makeInvoke({ create_project: projectDto() });
    const api = new TauriApi(invokeFn, toUrl);

    const project = await api.createProject({
      name: "Demo",
      size: { w: 1536, h: 1024, preset: "web" },
      brandBrief: "brand",
      styleBrief: "",
    });

    expect(calls[0]).toMatchObject({
      command: "create_project",
      args: {
        input: {
          name: "Demo",
          size: { w: 1536, h: 1024, preset: "web" },
          brandBrief: "brand",
          styleBrief: "",
        },
      },
    });
    expect(project.boardCandidates[0].url).toContain("candidates%2F0001.png");
    expect(project.anchor?.candidateId).toBe("0001");
    expect(project.pages[0].current?.candidateId).toBe("0002");
    expect(project.pages[0].history[0].ts).toBe(80);
  });

  it("generates the board via the SDK and records one image per item", async () => {
    stubAnchorFetch();
    const images = fakeImagesClient();
    const { calls, invokeFn } = makeInvoke({
      get_project: projectDtoNoAnchor(),
      update_board_brief: projectDtoNoAnchor(),
      get_cms_api_key: "sk-cms-test-dummy",
      get_generation_config: { baseUrl: "https://veren.top/api", model: "gpt-image-2" },
      record_generated_image: generateDto(),
    });
    const api = new TauriApi(invokeFn, toUrl, fakeImageClientFactory(images.client));

    const result = await api.generateBoard(
      "p1",
      {
        brandKeywords: "acme",
        colorDirection: "blue",
        fontMood: "serif",
        radiusDensity: "rounded",
        reference: "shot",
      },
      { count: 3 },
    );

    // SDK call: generations (no anchor yet), merged briefs, engine prompt.
    // (The injected client factory owns key/config reads; those are covered
    // in client.test.ts against the real factory.)
    expect(calls.map((c) => c.command)).toEqual([
      "get_project",
      "update_board_brief",
      "record_generated_image",
    ]);
    // C2-FE 偏差①: the merged brief is persisted BEFORE the SDK call (the
    // old generate_board order), so project.json keeps the amendment.
    expect(calls[1]).toMatchObject({
      command: "update_board_brief",
      args: {
        projectId: "p1",
        input: { brandBrief: "acme", styleBrief: "blue / serif / rounded | shot" },
      },
    });
    expect(images.calls.generate).toHaveLength(1);
    expect(images.calls.edit).toHaveLength(0);
    const [body] = images.calls.generate[0] as [Record<string, unknown>];
    expect(body).toMatchObject({ model: "gpt-image-2", n: 3, size: "1536x1024", quality: "low" });
    const prompt = String(body.prompt);
    expect(prompt).toContain('the product "Demo"');
    expect(prompt).toContain("Brand brief: acme");
    expect(prompt).toContain("Style brief: blue / serif / rounded | shot");
    expect(prompt).toContain("Constraints:");
    expect(prompt).toContain("- canvas-locked: compose for exactly 1536x1024");

    // Persistence: the b64 item rides imageBase64 into record_generated_image.
    expect(calls[2].args).toMatchObject({
      input: {
        projectId: "p1",
        kind: "board",
        target: "",
        endpoint: "generations",
        imageBase64: "AAAA",
        size: "1536x1024",
        quality: "low",
        model: "gpt-image-2",
      },
    });
    expect(calls[2].args).not.toHaveProperty("input.imageUrl");

    // Canvas refresh payload mapping + brief overlay for the session.
    expect(result.candidates[0].url).toContain("0003.png");
    expect(result.candidates[0].seed).toBe(8);
    expect(result.project.id).toBe("p1");
    expect(result.project.brandBrief).toBe("acme");
    expect(result.project.styleBrief).toBe("blue / serif / rounded | shot");
  });

  it("persists a standalone brief amendment via update_board_brief", async () => {
    const { calls, invokeFn } = makeInvoke({ update_board_brief: projectDto() });
    const api = new TauriApi(invokeFn, toUrl);

    const detail = await api.updateBoardBrief("p1", {
      brandBrief: " 新品牌 ",
      styleBrief: " 新风格 ",
    });

    expect(calls[0]).toMatchObject({
      command: "update_board_brief",
      args: {
        projectId: "p1",
        input: { brandBrief: " 新品牌 ", styleBrief: " 新风格 " },
      },
    });
    expect(detail.brandBrief).toBe("brand");
    expect(detail.pages).toHaveLength(1);
  });

  it("ramps onProgress toward 0.9 while the SDK job runs (edits, anchor first)", async () => {
    const fetchMock = stubAnchorFetch();
    const images = fakeImagesClient();
    const { release } = images.pendingEdit();
    let released = false;
    const invokeFn = (async (command: string) => {
      if (command === "get_project") return projectDto();
      if (command === "get_cms_api_key") return "sk-cms-test-dummy";
      if (command === "get_generation_config") {
        return { baseUrl: "https://veren.top/api", model: "gpt-image-2" };
      }
      if (command === "record_generated_image") {
        if (!released) throw new Error("record ran before the SDK call finished");
        return generateDto();
      }
      throw new Error(`unexpected command ${command}`);
    }) as unknown as InvokeFn;
    const api = new TauriApi(invokeFn, toUrl, fakeImageClientFactory(images.client));
    const progress: number[] = [];

    const pending = api.generatePage("p1", "dashboard", {
      count: 2,
      onProgress: (value) => progress.push(value),
    });
    // The ramp starts only after the project/reference awaits settle, so
    // advance in slices (each slice flushes the pending microtask chain).
    for (let slice = 0; slice < 12; slice += 1) {
      await vi.advanceTimersByTimeAsync(2_000);
    }
    expect(progress.length).toBeGreaterThan(0);
    expect(progress.every((value) => value > 0 && value <= 0.9)).toBe(true);

    // The anchor rides as reference image 1 (anchor-first edits flow).
    const [body, options] = images.calls.edit[0] as [Record<string, unknown>, { signal?: AbortSignal }];
    const refs = body.image as File[];
    expect(refs).toHaveLength(1);
    expect(refs[0].name).toBe("anchor.png");
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(body).toMatchObject({ model: "gpt-image-2", prompt: expect.any(String), n: 2 });
    expect(options.signal).toBeUndefined();

    released = true;
    release({ created: 1, data: [{ b64_json: "QQ==" }] });
    const result = await pending;
    expect(result.project.pages[0].slug).toBe("dashboard");
    expect(progress.at(-1)).toBe(1);
  });

  it("passes delete targets through and maps errors to ApiError", async () => {
    const { calls, invokeFn } = makeInvoke({ delete_artifact: projectDto() });
    const api = new TauriApi(invokeFn, toUrl);

    const project = await api.deleteArtifact("p1", { kind: "pageHistory", slug: "dashboard", ts: 80 });
    expect(calls[0].args).toEqual({
      projectId: "p1",
      target: { kind: "pageHistory", slug: "dashboard", ts: 80 },
    });
    expect(project.pages[0].slug).toBe("dashboard");

    const failing = new TauriApi(
      async () => {
        throw { code: "ANCHOR_REQUIRED", message: "no anchor yet", hint: "pick first" };
      },
      toUrl,
    );
    const error = await failing.pickAnchor("p1", "0001").catch((e: unknown) => e);
    expect(error).toBeInstanceOf(ApiError);
    expect(error).toMatchObject({ code: "ANCHOR_REQUIRED", message: "no anchor yet", hint: "pick first" });

    const unknownCode = new TauriApi(
      async () => {
        throw { code: "SOMETHING_ELSE", message: "boom" };
      },
      toUrl,
    );
    const fallback = await unknownCode.listProjects().catch((e: unknown) => e);
    expect(fallback).toMatchObject({ code: "UNKNOWN" });
  });

  it("passes update payloads to update_page/update_component", async () => {
    const { calls, invokeFn } = makeInvoke({
      update_page: projectDto(),
      update_component: projectDto(),
    });
    const api = new TauriApi(invokeFn, toUrl);

    await api.updatePage("p1", "dashboard", { brief: "grid v2" });
    await api.updateComponent("p1", "button-set", { brief: "three states v2" });

    expect(calls[0]).toMatchObject({
      command: "update_page",
      args: { projectId: "p1", slug: "dashboard", input: { brief: "grid v2" } },
    });
    expect(calls[1]).toMatchObject({
      command: "update_component",
      args: { projectId: "p1", name: "button-set", input: { brief: "three states v2" } },
    });
  });

  it("maps get_lineage records and passes null through", async () => {
    const record = {
      at: "2026-09-09T00:00:00Z",
      endpoint: "edits",
      prompt: "Image 1 is the board",
      params: { model: "gpt-image-2", size: "1536x1024", quality: "low", n: 2, seed: 7 },
      candidateIds: ["0002"],
      source: "agent-file",
      templateId: "page-ui-standard",
    };
    const { calls, invokeFn } = makeInvoke({ get_lineage: record });
    const api = new TauriApi(invokeFn, toUrl);

    const found = await api.getLineage("p1", {
      kind: "page",
      slug: "dashboard",
      candidateId: "0002",
    });
    expect(calls[0]).toMatchObject({
      command: "get_lineage",
      args: { projectId: "p1", target: { kind: "page", slug: "dashboard", candidateId: "0002" } },
    });
    expect(found).toEqual(record);

    const missing = new TauriApi(makeInvoke({ get_lineage: null }).invokeFn, toUrl);
    expect(await missing.getLineage("p1", { kind: "board", candidateId: "9999" })).toBeNull();
  });

  it("maps export_project results and passes the jobId", async () => {
    const { calls, invokeFn } = makeInvoke({
      export_project: {
        outDir: "/home/u/Rudder/exports/Demo",
        files: [{ path: "board/anchor.png", bytes: 9, kind: "image" }],
      },
    });
    const api = new TauriApi(invokeFn, toUrl);

    const result = await api.exportProject("p1", "~/Rudder/exports/Demo");
    expect(calls[0].args).toMatchObject({
      projectId: "p1",
      outDir: "~/Rudder/exports/Demo",
      options: { jobId: expect.any(String) },
    });
    expect(result).toEqual({
      outDir: "/home/u/Rudder/exports/Demo",
      files: [{ path: "board/anchor.png", bytes: 9, kind: "image" }],
    });
  });
});
