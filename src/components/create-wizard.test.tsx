// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import { CreateWizard } from "@/components/create-wizard";
import i18n from "@/i18n";
import { StudioProvider } from "@/state/studio";
import { ToastProvider } from "@/state/toast";

/**
 * P2 向导生成体验：JobPanel 姿态（进度+耗时+取消，取消后无孤儿作业）、
 * 段标题三态（THEME §5：done=苍青✓ / current=缃色圆点+加粗 / future=muted）、
 * 「✓ 锚点」标记 ≥11px。全部走真实 MockApi（浏览器环境自动降级 Mock）。
 */
function renderWizard() {
  return render(
    <ToastProvider>
      <StudioProvider>
        <CreateWizard open onClose={() => {}} />
      </StudioProvider>
    </ToastProvider>,
  );
}

function fillInput(testId: string, value: string) {
  fireEvent.change(screen.getByTestId(testId), { target: { value } });
}

/** Step 1 -> Step 2: create the project record (Mock resolves immediately). */
async function createProject() {
  fillInput("wizard-name-input", "演示项目");
  fireEvent.click(screen.getByTestId("wizard-create"));
  await screen.findByTestId("wizard-generate");
}

/** Drain pending promise continuations (abort/cancel chains, instant ops). */
async function flushMicrotasks() {
  await act(async () => {
    for (let i = 0; i < 10; i += 1) {
      await Promise.resolve();
    }
  });
}

beforeAll(async () => {
  await i18n.changeLanguage("zh-CN");
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("create wizard generation experience (P2)", () => {
  it("board generation shows the JobPanel pose; cancel returns to the empty state with no orphan job", async () => {
    renderWizard();
    await screen.findByTestId("wizard-name-input");
    await createProject();
    fillInput("wizard-brand", "苍青主色，缃色点缀");

    vi.useFakeTimers();
    fireEvent.click(screen.getByTestId("wizard-generate"));

    // Waiting pose matches the regen drawer: progress + elapsed + eta + cancel.
    const panel = screen.getByTestId("job-panel");
    expect(panel.getAttribute("data-job-kind")).toBe("board");
    expect(screen.getByRole("progressbar")).toBeTruthy();
    expect(screen.getByTestId("job-cancel")).toBeTruthy();
    expect(screen.getByText("生成中，约需 30 秒~3 分钟")).toBeTruthy();

    fireEvent.click(screen.getByTestId("job-cancel"));
    await flushMicrotasks();

    // Back to the candidate-empty state, no job left running.
    expect(screen.queryByTestId("job-panel")).toBeNull();
    expect((screen.getByTestId("wizard-generate") as HTMLButtonElement).disabled).toBe(false);
    expect(screen.queryByTestId("wizard-candidates")).toBeNull();
    expect(screen.getByText("操作已取消。")).toBeTruthy();
  });

  it("generation completes into candidates and the anchor mark is 11px", async () => {
    renderWizard();
    await screen.findByTestId("wizard-name-input");
    await createProject();
    fillInput("wizard-brand", "苍青主色，缃色点缀");

    vi.useFakeTimers();
    fireEvent.click(screen.getByTestId("wizard-generate"));
    expect(screen.getByTestId("job-panel")).toBeTruthy();

    // Mock board generation takes 2..5s; run the fake clock past it.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });

    expect(screen.queryByTestId("job-panel")).toBeNull();
    const grid = screen.getByTestId("wizard-candidates");
    fireEvent.click(within(grid).getAllByRole("button")[0]);
    await flushMicrotasks();

    const badge = within(screen.getByTestId("wizard-candidates")).getByText("锚点");
    expect(badge.className).toContain("text-[11px]");
    expect(screen.getByTestId("wizard-section-2").getAttribute("data-section-state")).toBe("done");
  });

  it("section titles cover the THEME §5 tri-state with exactly one current section", async () => {
    const { container } = renderWizard();
    await screen.findByTestId("wizard-name-input");

    // Step 1 = current: accent dot + bold label.
    const first = screen.getByTestId("wizard-section-1");
    expect(first.getAttribute("data-section-state")).toBe("current");
    expect(first.className).toContain("font-bold");
    expect(first.querySelector("span.bg-accent")).toBeTruthy();

    await createProject();
    expect(screen.getByTestId("wizard-section-1").getAttribute("data-section-state")).toBe("done");
    const second = screen.getByTestId("wizard-section-2");
    expect(second.getAttribute("data-section-state")).toBe("current");
    expect(second.className).toContain("font-bold");
    expect(second.querySelector("span.bg-accent")).toBeTruthy();
    expect(container.querySelectorAll('[data-section-state="current"]')).toHaveLength(1);

    // Skip to step 3: the passed-over section 2 falls back to the muted future state.
    fireEvent.click(screen.getByTestId("wizard-skip"));
    const secondAfter = screen.getByTestId("wizard-section-2");
    expect(secondAfter.getAttribute("data-section-state")).toBe("future");
    expect(secondAfter.className).not.toContain("font-bold");
    const third = screen.getByTestId("wizard-section-3");
    expect(third.getAttribute("data-section-state")).toBe("current");
    expect(third.querySelector("span.bg-accent")).toBeTruthy();
    expect(container.querySelectorAll('[data-section-state="current"]')).toHaveLength(1);
  });
});
