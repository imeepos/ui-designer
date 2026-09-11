import { describe, expect, it } from "vitest";

import {
  ANCHOR_REFERENCE,
  composeBoardPrompt,
  composeComponentPrompt,
  composePagePrompt,
  extractSlots,
  extractVerbatimLabels,
  fillTemplate,
  renderConstraints,
} from "@/lib/generation/prompt";

const project = {
  name: "样例产品",
  brandBrief: "远洋航运，面向船东的 B 端工具",
  styleBrief: "海军蓝 + 黄铜点缀，圆角 8px",
  canvasW: 1536,
  canvasH: 1024,
};

describe("fillTemplate", () => {
  it("mirrors the Rust rules: drop all-empty lines, collapse blanks, trim", () => {
    const prompt = fillTemplate(
      { id: "t", appliesTo: "board", skeleton: "A: {x}\nB: {y}\nfixed\n\n\n\nZ" },
      { x: "kept", y: "" },
    );
    expect(prompt).toBe("A: kept\nfixed\n\nZ");
  });

  it("rejects unknown slots like the engine (exit-1 shape)", () => {
    expect(() => fillTemplate({ id: "t", appliesTo: "board", skeleton: "{missing}" }, {})).toThrow(
      /slot `\{missing\}`/,
    );
  });

  it("extractSlots reads only [A-Za-z0-9_.] tokens", () => {
    expect(extractSlots("{a.b} {} {x-y} {c_d}")).toEqual(["a.b", "c_d"]);
  });
});

describe("extractVerbatimLabels", () => {
  it("splits Labels: markers and dedupes", () => {
    const { labels, kept } = extractVerbatimLabels("导航 5 项。Labels: 概览|报表|概览");
    expect(labels).toEqual(["概览", "报表"]);
    expect(kept).toBe("导航 5 项。");
  });
});

describe("renderConstraints", () => {
  it("substitutes the canvas and applies only the kind's rows", () => {
    const block = renderConstraints("board", project);
    expect(block).toContain("- canvas-locked: compose for exactly 1536x1024");
    expect(block).toContain("- board-cohesion:");
    expect(block).not.toContain("- consistency-first:");
    expect(block).not.toContain("- identity-lock:");
  });

  it("appends project negative hints to explicit-negatives", () => {
    const block = renderConstraints("page", { ...project, negativeHints: ["无渐变"] });
    expect(block).toContain("additionally forbidden per project: 无渐变");
  });
});

describe("composeBoardPrompt", () => {
  it("fills the board skeleton and appends constraints", () => {
    const prompt = composeBoardPrompt(project);
    expect(prompt).toContain('a UI design system board (风格总板) for the product "样例产品"');
    expect(prompt).toContain("Brand brief: 远洋航运，面向船东的 B 端工具");
    expect(prompt).toContain("Style brief: 海军蓝 + 黄铜点缀，圆角 8px");
    expect(prompt).toContain("Canvas: 1536x1024");
    expect(prompt).toContain("1. COLOR PALETTE");
    expect(prompt).toContain("\nConstraints:\n- text-hardcode:");
    expect(prompt).toContain("- small-size-legibility:");
  });
});

describe("composePagePrompt", () => {
  it("opens with the anchor reference, renders invariants and labels", () => {
    const prompt = composePagePrompt(project, {
      slug: "dashboard",
      brief: "顶部导航 5 项。Labels: 概览|报表",
    });
    expect(prompt.startsWith(`${ANCHOR_REFERENCE}。严格沿用它的设计语言`)).toBe(true);
    expect(prompt).toContain('design the full "dashboard" page');
    expect(prompt).toContain("Layout brief: 顶部导航 5 项。");
    // Invariants block rendered from project data.
    expect(prompt).toContain("Invariants (do NOT change): strictly reuse Image 1's exact");
    expect(prompt).toContain("- color palette (same hex values for primary/background/text/accent),");
    expect(prompt).toContain("Overall mood stays: 海军蓝 + 黄铜点缀，圆角 8px");
    expect(prompt).toContain("Only compose NEW layout/content; never redesign the system.");
    // Verbatim labels travel as a dedicated constraint line.
    expect(prompt).toContain('Verbatim labels: the following must appear exactly as written');
    expect(prompt).toContain('"概览" "报表"');
    expect(prompt).toContain("- consistency-first:");
    // Board-only rows stay out.
    expect(prompt).not.toContain("- board-cohesion:");
  });
});

describe("composeComponentPrompt", () => {
  it("keeps the same-family identity lock and label constraint", () => {
    const prompt = composeComponentPrompt(project, {
      name: "button-set",
      kind: "buttons",
      brief: "3 变体 × 5 状态。Labels: 提交|取消",
    });
    expect(prompt).toContain("component detail sheet for the \"buttons\" family");
    expect(prompt).toContain("same-family identity lock");
    expect(prompt).toContain('"提交" "取消"');
    expect(prompt).toContain("- identity-lock:");
  });
});
