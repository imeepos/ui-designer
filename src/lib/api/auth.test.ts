import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { invoke } from "@tauri-apps/api/core";

import { ApiError } from "@/lib/api/types";
import {
  fetchMe,
  fetchSessionStatus,
  login,
  logout,
  register,
  type RudderSession,
  type RudderUser,
} from "@/lib/api/auth";

// auth.ts calls the real `invoke`; the desktop shell is absent in vitest, so
// the module is mocked and the double records calls / answers from here.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

function sessionDto(): RudderSession {
  const user: RudderUser = {
    id: "u-1",
    username: "helmsman",
    email: null,
    role: "user",
    status: "active",
    credits: 90,
    createdAt: "2026-09-10T00:00:00Z",
  };
  return { token: "jwt-token-value", user };
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

  it("reports no session without invoking the bridge", async () => {
    stubDesktop(false);
    await expect(fetchSessionStatus()).resolves.toEqual({ hasToken: false });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it.each(["login", "register", "fetchMe", "logout"] as const)(
    "rejects %s with NOT_IMPLEMENTED outside the desktop shell",
    async (fn) => {
      stubDesktop(false);
      const call =
        fn === "login"
          ? login("helmsman", "secret-1")
          : fn === "register"
            ? register("helmsman", "secret-1")
            : fn === "fetchMe"
              ? fetchMe()
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

  it("login invokes auth_login with the credentials input", async () => {
    invokeMock.mockResolvedValue(sessionDto());
    const session = await login("helmsman", "secret-1");
    expect(invokeMock).toHaveBeenCalledWith("auth_login", {
      input: { username: "helmsman", password: "secret-1" },
    });
    expect(session.user.username).toBe("helmsman");
    expect(session.user.credits).toBe(90);
  });

  it("register passes the optional email (null when absent)", async () => {
    invokeMock.mockResolvedValue(sessionDto());
    await register("helmsman", "secret-1", "u@example.com");
    expect(invokeMock).toHaveBeenCalledWith("auth_register", {
      input: { username: "helmsman", password: "secret-1", email: "u@example.com" },
    });
    await register("helmsman", "secret-1");
    expect(invokeMock).toHaveBeenLastCalledWith("auth_register", {
      input: { username: "helmsman", password: "secret-1", email: null },
    });
  });

  it("fetchMe and logout hit auth_me / clear_session_token", async () => {
    invokeMock.mockResolvedValue(sessionDto().user);
    const user = await fetchMe();
    expect(invokeMock).toHaveBeenCalledWith("auth_me");
    expect(user.id).toBe("u-1");

    invokeMock.mockResolvedValue(undefined);
    await expect(logout()).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenCalledWith("clear_session_token");
  });

  it("fetchSessionStatus reads the shell answer", async () => {
    invokeMock.mockResolvedValue({ hasToken: true });
    await expect(fetchSessionStatus()).resolves.toEqual({ hasToken: true });
    expect(invokeMock).toHaveBeenCalledWith("get_session_status");
  });

  it("maps rust error envelopes into ApiError", async () => {
    invokeMock.mockRejectedValue({
      code: "VALIDATION_ERROR",
      message: "username taken",
      hint: "pick another",
    });
    await expect(login("helmsman", "secret-1")).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
      message: "username taken",
      hint: "pick another",
    } satisfies Partial<ApiError>);
  });
});
