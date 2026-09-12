// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsDialog } from "@/components/settings-dialog";
import i18n from "@/i18n";
import { ApiError } from "@/lib/api/types";
import type { AuthStatus, CmsSession } from "@/lib/api/auth";
import { ToastProvider } from "@/state/toast";

// Only the network-shaped calls are doubled; the real module keeps
// `isSessionExpiredError` (the expired-cookie classifier) authoritative.
vi.mock("@/lib/api/auth", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api/auth")>();
  return {
    ...actual,
    fetchAuthStatus: vi.fn(),
    login: vi.fn(),
    logout: vi.fn(),
    register: vi.fn(),
    fetchAccount: vi.fn(),
  };
});

import { fetchAuthStatus, login } from "@/lib/api/auth";

const fetchAuthStatusMock = vi.mocked(fetchAuthStatus);
const loginMock = vi.mocked(login);

function sessionDto(): CmsSession {
  return {
    user: {
      id: 11,
      email: "fan@example.com",
      name: "舵手",
      disabled: false,
      created_at: 1_700_000_000,
    },
    balance: 4200,
  };
}

function signedOut(): AuthStatus {
  return { loggedIn: false, account: null };
}

function renderDialog() {
  return render(
    <ToastProvider>
      <SettingsDialog open onClose={() => {}} />
    </ToastProvider>,
  );
}

function fillEmailAndPassword(): void {
  fireEvent.change(screen.getByTestId("settings-email"), {
    target: { value: "fan@example.com" },
  });
  fireEvent.change(screen.getByTestId("settings-password"), {
    target: { value: "secret-1" },
  });
}

beforeAll(async () => {
  await i18n.changeLanguage("zh-CN");
});

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe("settings dialog account section (cms chain)", () => {
  beforeEach(() => {
    fetchAuthStatusMock.mockResolvedValue(signedOut());
  });

  it("signed out + form validation errors: empty submit never calls the bridge", async () => {
    renderDialog();
    const form = await screen.findByTestId("settings-account-form");
    expect(form).toBeTruthy();

    fireEvent.click(screen.getByTestId("settings-login"));
    const error = await screen.findByTestId("settings-account-error");
    expect(error.textContent).toContain("请输入邮箱");
    expect(loginMock).not.toHaveBeenCalled();

    // Email filled but password missing → password validation error.
    fireEvent.change(screen.getByTestId("settings-email"), {
      target: { value: "fan@example.com" },
    });
    fireEvent.click(screen.getByTestId("settings-login"));
    await waitFor(() => {
      expect(screen.getByTestId("settings-account-error").textContent).toContain("请输入密码");
    });
    expect(loginMock).not.toHaveBeenCalled();
  });

  it("signing in: the busy copy shows and the submit stays disabled", async () => {
    let resolveLogin!: (session: CmsSession) => void;
    loginMock.mockImplementation(
      () => new Promise<CmsSession>((resolve) => { resolveLogin = resolve; }),
    );
    renderDialog();
    await screen.findByTestId("settings-account-form");
    fillEmailAndPassword();

    fireEvent.click(screen.getByTestId("settings-login"));
    const submit = await screen.findByTestId("settings-login");
    await waitFor(() => {
      expect(submit.textContent).toContain("登录中");
      expect((submit as HTMLButtonElement).disabled).toBe(true);
    });
    expect(loginMock).toHaveBeenCalledWith("fan@example.com", "secret-1");

    await act(async () => {
      resolveLogin(sessionDto());
    });
    await waitFor(() => {
      expect(screen.getByTestId("settings-account-user")).toBeTruthy();
    });
  });

  it("signed in: name, email and the points balance are visible", async () => {
    fetchAuthStatusMock.mockResolvedValue({
      loggedIn: true,
      account: { user: sessionDto().user, balance: 4200 },
    });
    renderDialog();
    const card = await screen.findByTestId("settings-account-user");
    expect(card).toBeTruthy();
    expect(screen.getByTestId("settings-account-name").textContent).toBe("舵手");
    expect(screen.getByTestId("settings-account-email").textContent).toBe("fan@example.com");
    expect(screen.getByTestId("settings-account-balance").textContent).toBe("4200");
    expect(loginMock).not.toHaveBeenCalled();
  });

  it("expired cookie: the re-login guidance replaces the account card", async () => {
    fetchAuthStatusMock.mockRejectedValue(
      new ApiError("SESSION_EXPIRED", "1003: authentication required"),
    );
    renderDialog();
    const banner = await screen.findByTestId("settings-session-expired");
    expect(banner.textContent).toContain("登录已过期");
    // The form is right there so the user can sign in again immediately.
    expect(screen.getByTestId("settings-account-form")).toBeTruthy();
    expect(screen.queryByTestId("settings-account-user")).toBeNull();
  });

  it("login failure: inline error shows message and hint as two lines (findings M8)", async () => {
    loginMock.mockRejectedValue(
      new ApiError(
        "NOT_IMPLEMENTED",
        "account sign-in needs the desktop shell",
        "launch the Tauri app to sign in",
      ),
    );
    renderDialog();
    await screen.findByTestId("settings-account-form");
    fillEmailAndPassword();
    fireEvent.click(screen.getByTestId("settings-login"));

    const error = await screen.findByTestId("settings-account-error");
    await waitFor(() => {
      expect(error.textContent).toContain("该能力尚未接入核心库");
    });
    // Hint comes from the same errors.* source as the toast (weakened line).
    expect(error.textContent).toContain("当前为 Mock 预览");
    expect(error.querySelectorAll("p")).toHaveLength(2);
  });
});
