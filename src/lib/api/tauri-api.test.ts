import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/lib/api/types";
import { TauriApi, type GeneratePayloadDto, type InvokeFn, type ProjectDetailDto } from "@/lib/api/tauri-api";

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

  it("invokes generate_board with a jobId and maps the payload", async () => {
    const { calls, invokeFn } = makeInvoke({ generate_board: generateDto() });
    const api = new TauriApi(invokeFn, toUrl);

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

    expect(calls[0].command).toBe("generate_board");
    expect(calls[0].args).toMatchObject({
      projectId: "p1",
      brief: { brandKeywords: "acme", reference: "shot" },
      options: { count: 3, jobId: expect.any(String) },
    });
    expect(result.candidates[0].url).toContain("0003.png");
    expect(result.candidates[0].seed).toBe(8);
    expect(result.project.id).toBe("p1");
  });

  it("ramps onProgress toward 0.9 while the job runs", async () => {
    let release!: (value: GeneratePayloadDto) => void;
    const invokeFn = vi.fn(
      (_command: string, _args?: Record<string, unknown>) =>
        new Promise<GeneratePayloadDto>((resolve) => {
          release = resolve;
        }),
    ) as unknown as InvokeFn;
    const api = new TauriApi(invokeFn, toUrl);
    const progress: number[] = [];

    const pending = api.generatePage("p1", "dashboard", {
      count: 2,
      onProgress: (value) => progress.push(value),
    });
    await vi.advanceTimersByTimeAsync(24_000);
    expect(progress.length).toBeGreaterThan(0);
    expect(progress.every((value) => value > 0 && value <= 0.9)).toBe(true);

    release(generateDto());
    const result = await pending;
    expect(result.project.pages[0].slug).toBe("dashboard");
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
