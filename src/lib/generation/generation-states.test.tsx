// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { JobPanel } from "@/components/job-panel";
import i18n from "@/i18n";
import { ApiError } from "@/lib/api/types";
import { OPEN_SETTINGS_EVENT } from "@/lib/events";
import type { ActiveJob } from "@/state/studio";
import { StudioProvider } from "@/state/studio";
import { ToastProvider, useToast } from "@/state/toast";

/** Fires a toast error from inside the provider tree on click. */
function ErrorProbe({ error }: { error: unknown }) {
  const toast = useToast();
  return (
    <button type="button" data-testid="fire" onClick={() => toast.error(error)}>
      fire
    </button>
  );
}

async function renderError(error: unknown) {
  render(
    <ToastProvider>
      <ErrorProbe error={error} />
    </ToastProvider>,
  );
  fireEvent.click(screen.getByTestId("fire"));
  await waitFor(() => expect(screen.getByTestId("toast-error")).toBeTruthy());
}

describe("generation failure toasts (SDK 直连错误形 → 专属文案)", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("zh-CN");
  });

  afterEach(cleanup);

  it("QUOTA_EXCEEDED shows the recharge/admin guidance, not a raw error", async () => {
    // Mirror mapSdkError: message from the gateway + hint marker
    // (the toast renders the localized errors.QUOTA_EXCEEDED.hint).
    await renderError(
      new ApiError("QUOTA_EXCEEDED", "积分不足：本次生成需要 10 点", "top up"),
    );
    expect(screen.getByText("点数余额不足，本次生成未完成。")).toBeTruthy();
    expect(screen.getByText(/充值后重试，或联系管理员/)).toBeTruthy();
    expect(screen.getByText("QUOTA_EXCEEDED")).toBeTruthy();
  });

  it("NO_CREDENTIALS guides to the account section with an open-settings action", async () => {
    const listener = vi.fn();
    window.addEventListener(OPEN_SETTINGS_EVENT, listener);
    try {
      await renderError(
      new ApiError("NO_CREDENTIALS", "no cms api key", "sign in first"),
    );
      expect(screen.getByText("尚未登录账户。")).toBeTruthy();
      expect(screen.getByText(/登录后重试/)).toBeTruthy();
      // The inline action routes to the settings dialog (account form).
      fireEvent.click(screen.getByRole("button", { name: "打开设置" }));
      expect(listener).toHaveBeenCalledTimes(1);
    } finally {
      window.removeEventListener(OPEN_SETTINGS_EVENT, listener);
    }
  });
});

describe("generation in-progress UI (生成中)", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("zh-CN");
  });

  afterEach(cleanup);

  it("renders the job panel with progress and cancel while generating", () => {
    const job: ActiveJob = { kind: "board", progress: 0.35, startedAt: Date.now() };
    render(
      <ToastProvider>
        <StudioProvider>
          <JobPanel job={job} />
        </StudioProvider>
      </ToastProvider>,
    );
    expect(screen.getByTestId("job-panel")).toBeTruthy();
    const bar = screen.getByRole("progressbar");
    expect(bar.getAttribute("aria-valuenow")).toBe("35");
    expect(screen.getByTestId("job-cancel")).toBeTruthy();
  });
});
