import { describe, expect, test } from "vitest";

import { MockApi } from "@/lib/api/mock-api";
import { ApiError } from "@/lib/api/types";
import { validateCanvasSize } from "@/lib/size";

describe("MockApi four-step flow", () => {
  // Compressed delays so the whole flow fits the default test timeout.
  const api = new MockApi({ min: 5, max: 15 });

  test("create -> board -> anchor -> page -> component -> export", async () => {
    const project = await api.createProject({
      name: "Smoke",
      size: { w: 1536, h: 1024, preset: "web" },
      brandBrief: "navy studio",
    });
    expect(project.id).toBeTruthy();
    expect(project.anchor).toBeNull();

    const empty = await api.listProjects();
    expect(empty).toHaveLength(1);

    const board = await api.generateBoard(
      project.id,
      {
        brandKeywords: "navy, brass",
        colorDirection: "deep navy",
        fontMood: "Inter",
        radiusDensity: "12px",
        reference: "",
      },
      { count: 3, onProgress: () => {} },
    );
    expect(board.candidates).toHaveLength(3);
    expect(board.project.boardCandidates).toHaveLength(3);
    expect(board.candidates[0].filter).toBeUndefined();
    expect(board.candidates[1].filter).toBeDefined();

    const withAnchor = await api.pickAnchor(project.id, board.candidates[2].id);
    expect(withAnchor.anchor?.candidateId).toBe(board.candidates[2].id);

    // Pages require an anchor; before that the adapter must reject.
    const noAnchorApi = new MockApi();
    const orphan = await noAnchorApi.createProject({
      name: "Orphan",
      size: { w: 1536, h: 1024, preset: "web" },
      brandBrief: "",
    });
    await noAnchorApi.addPage(orphan.id, { slug: "gallery", brief: "grid" });
    await expect(
      noAnchorApi.generatePage(orphan.id, "gallery", { count: 1 }),
    ).rejects.toMatchObject({ code: "ANCHOR_REQUIRED" } satisfies Partial<ApiError>);

    await api.addPage(project.id, { slug: "gallery", brief: "grid + detail" });
    const page = await api.generatePage(project.id, "gallery", { count: 2 });
    expect(page.candidates).toHaveLength(2);

    const picked = await api.pickPage(project.id, "gallery", page.candidates[0].id);
    const gallery = picked.pages.find((item) => item.slug === "gallery");
    expect(gallery?.current?.candidateId).toBe(page.candidates[0].id);
    expect(gallery?.candidates).toHaveLength(0);

    await api.addComponent(project.id, {
      name: "button-set",
      type: "buttons",
      brief: "three states",
    });
    const component = await api.generateComponent(project.id, "button-set", { count: 1 });
    const pickedComponent = await api.pickComponent(
      project.id,
      "button-set",
      component.candidates[0].id,
    );
    expect(
      pickedComponent.components.find((item) => item.name === "button-set")?.current,
    ).not.toBeNull();

    const exported = await api.exportProject(project.id, "/tmp/rudder-mock");
    const paths = exported.files.map((file) => file.path);
    expect(paths).toContain("board/anchor.png");
    expect(paths).toContain("pages/gallery/current.png");
    expect(paths).toContain("components/button-set/current.png");
    expect(paths).toContain("manifest.json");
    expect(paths).toContain("PROMPTS.md");
  });

  test("update ops persist amended briefs and validate input", async () => {
    const api = new MockApi({ min: 5, max: 15 });
    const project = await api.createProject({
      name: "Update",
      size: { w: 1536, h: 1024, preset: "web" },
      brandBrief: "",
    });
    await api.addPage(project.id, { slug: "gallery", brief: "grid" });
    await api.addComponent(project.id, {
      name: "button-set",
      type: "buttons",
      brief: "three states",
    });

    const pageUpdated = await api.updatePage(project.id, "gallery", { brief: "grid v2" });
    expect(pageUpdated.pages.find((item) => item.slug === "gallery")?.brief).toBe("grid v2");

    const componentUpdated = await api.updateComponent(project.id, "button-set", {
      brief: "three states v2",
    });
    expect(
      componentUpdated.components.find((item) => item.name === "button-set")?.brief,
    ).toBe("three states v2");

    await expect(
      api.updatePage(project.id, "gallery", { brief: "   " }),
    ).rejects.toMatchObject({ code: "VALIDATION_ERROR" } satisfies Partial<ApiError>);
    await expect(
      api.updatePage(project.id, "missing", { brief: "x" }),
    ).rejects.toMatchObject({ code: "NOT_FOUND" } satisfies Partial<ApiError>);
    await expect(
      api.updateComponent(project.id, "missing", { brief: "x" }),
    ).rejects.toMatchObject({ code: "NOT_FOUND" } satisfies Partial<ApiError>);
  });

  test("rejects duplicates, anchor deletion and missing export dir", async () => {
    const api = new MockApi({ min: 5, max: 15 });
    const project = await api.createProject({
      name: "Guard",
      size: { w: 1536, h: 1024, preset: "web" },
      brandBrief: "",
    });

    await api.addPage(project.id, { slug: "gallery", brief: "grid" });
    await expect(
      api.addPage(project.id, { slug: "gallery", brief: "again" }),
    ).rejects.toMatchObject({ code: "DUPLICATE_SLUG" });

    await expect(
      api.deleteArtifact(project.id, { kind: "pageCurrent", slug: "gallery" }),
    ).rejects.toMatchObject({ code: "NOT_FOUND" });

    const board = await api.generateBoard(
      project.id,
      { brandKeywords: "x", colorDirection: "", fontMood: "", radiusDensity: "", reference: "" },
      { count: 1 },
    );
    await api.pickAnchor(project.id, board.candidates[0].id);
    await expect(
      api.deleteArtifact(project.id, {
        kind: "boardCandidate",
        candidateId: board.candidates[0].id,
      }),
    ).rejects.toMatchObject({ code: "ANCHOR_LOCKED" });

    await expect(api.exportProject(project.id, "")).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
    });
  });

  test("simulateWork respects abort and reports CANCELLED", async () => {
    const api = new MockApi({ min: 5, max: 15 });
    const project = await api.createProject({
      name: "Abort",
      size: { w: 1536, h: 1024, preset: "web" },
      brandBrief: "",
    });
    const controller = new AbortController();
    const promise = api.generateBoard(
      project.id,
      { brandKeywords: "x", colorDirection: "", fontMood: "", radiusDensity: "", reference: "" },
      { count: 1, signal: controller.signal },
    );
    controller.abort();
    await expect(promise).rejects.toMatchObject({ code: "CANCELLED" });
  });
});

describe("canvas size validation", () => {
  test("presets pass and custom enforces gpt-image-2 constraints", () => {
    expect(validateCanvasSize("web", "", "")).toMatchObject({ ok: true });
    expect(validateCanvasSize("desktop", "", "")).toMatchObject({ ok: true });

    expect(validateCanvasSize("custom", "16", "16")).toMatchObject({ ok: false, code: "pixels" });
    expect(validateCanvasSize("custom", "1537", "1024")).toMatchObject({
      ok: false,
      code: "multiple",
    });
    expect(validateCanvasSize("custom", "3104", "1024")).toMatchObject({
      ok: false,
      code: "aspect",
    });
    expect(validateCanvasSize("custom", "3072", "1024")).toMatchObject({ ok: true });
    expect(validateCanvasSize("custom", "1536", "1024")).toMatchObject({ ok: true });
    expect(validateCanvasSize("custom", "abc", "1024")).toMatchObject({
      ok: false,
      code: "required",
    });
  });
});
