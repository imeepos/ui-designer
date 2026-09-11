import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { invoke } from "@tauri-apps/api/core";

import { ApiError } from "@/lib/api/types";
import {
  fetchAccount,
  fetchAuthStatus,
  isSessionExpiredError,
  login,
  logout,
  register,
  type CmsSession,
  type CmsUser,
} from "@/lib/api/auth";

// auth.ts calls the real `invoke`; the desktop shell is absent in vitest, so
// the module is mocked and the double records calls / answers from here.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

function cmsUser(): CmsUser {
  return {
    id: 11,
    email: "fan@example.com",
    name: "舵手",
    disabled: false,
    created_at: 1_700_000_000,
  };
}

function sessionDto(): CmsSession {
  return { user: cmsUser(), balance: 4200 };
}

/** No `window` in the node test env → the browser (mock preview) branch. */
function stubDesktop(desktop: boolean): void {
  if (desktop) {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  }
  // else: leave globalThis without `window`.
}

describe("auth api (browser preview degradation)", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.unstubAllGlobals();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("reports signed out without invoking the bridge", async () => {
    stubDesktop(false);
    await expect(fetchAuthStatus()).resolves.toEqual({ loggedIn: false, account: null });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it.each(["login", "register", "fetchAccount", "logout"] as const)(
    "rejects %s with NOT_IMPLEMENTED outside the desktop shell",
    async (fn) => {
      stubDesktop(false);
      const call =
        fn === "login"
          ? login("fan@example.com", "secret-1")
          : fn === "register"
            ? register("fan@example.com", "secret-1", "舵手")
            : fn === "fetchAccount"
              ? fetchAccount()
              : logout();
      await expect(call).rejects.toMatchObject({
        code: "NOT_IMPLEMENTED",
      } satisfies Partial<ApiError>);
      expect(invokeMock).not.toHaveBeenCalled();
    },
  );
});

describe("auth api (desktop bridge)", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("login invokes auth_login with the cms credentials input", async () => {
    invokeMock.mockResolvedValue(sessionDto());
    const session = await login("fan@example.com", "secret-1");
    expect(invokeMock).toHaveBeenCalledWith("auth_login", {
      input: { email: "fan@example.com", password: "secret-1" },
    });
    expect(session.user.email).toBe("fan@example.com");
    expect(session.user.name).toBe("舵手");
    expect(session.balance).toBe(4200);
  });

  it("register invokes auth_register with the cms shape (no username)", async () => {
    invokeMock.mockResolvedValue(cmsUser());
    const user = await register("fan@example.com", "secret-1", "舵手");
    expect(invokeMock).toHaveBeenCalledWith("auth_register", {
      input: { email: "fan@example.com", password: "secret-1", name: "舵手" },
    });
    expect(user.id).toBe(11);
  });

  it("fetchAccount and logout hit auth_me / auth_logout", async () => {
    invokeMock.mockResolvedValue({ user: cmsUser(), balance: 4200 });
    const account = await fetchAccount();
    expect(invokeMock).toHaveBeenCalledWith("auth_me");
    expect(account.balance).toBe(4200);

    invokeMock.mockResolvedValue(undefined);
    await expect(logout()).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenCalledWith("auth_logout");
  });

  it("fetchAuthStatus reads the shell answer", async () => {
    invokeMock.mockResolvedValue({ loggedIn: true, account: { user: cmsUser(), balance: 1 } });
    await expect(fetchAuthStatus()).resolves.toMatchObject({ loggedIn: true });
    expect(invokeMock).toHaveBeenCalledWith("auth_status");
  });

  it("maps rust error envelopes into ApiError", async () => {
    invokeMock.mockRejectedValue({
      code: "API_ERROR",
      message: "1001: 邮箱或密码错误",
      hint: "check the credentials",
    });
    await expect(login("fan@example.com", "wrong")).rejects.toMatchObject({
      code: "API_ERROR",
      message: "1001: 邮箱或密码错误",
      hint: "check the credentials",
    } satisfies Partial<ApiError>);
  });

  it("flags session-expired envelopes for the re-login guidance", async () => {
    const expired = new ApiError("SESSION_EXPIRED", "1003: authentication required");
    expect(isSessionExpiredError(expired)).toBe(true);
    expect(isSessionExpiredError(new ApiError("API_ERROR", "1000: 邮箱已被注册"))).toBe(false);
    expect(isSessionExpiredError(new Error("plain"))).toBe(false);
  });
});
