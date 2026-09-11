import { invoke } from "@tauri-apps/api/core";

import { hasTauriRuntime } from "@/lib/api";
import { toApiError } from "@/lib/api/tauri-api";
import { ApiError, isApiError } from "@/lib/api/types";

/**
 * cms account API (Settings dialog "Account" section): register / sign in /
 * sign out / read the account snapshot against the cms service through
 * dedicated Tauri commands (`rudder-core::cms_auth`).
 *
 * The session cookie and the minted image key are persisted ONLY in the OS
 * keychain on the Rust side; no secret value is ever returned here — the
 * answers carry profile + points balance only. In the browser mock preview
 * these calls degrade gracefully: the status read reports "signed out",
 * mutations reject NOT_IMPLEMENTED.
 */

/** Public cms profile (`cms_auth::CmsUser`); timestamps are unix seconds. */
export interface CmsUser {
  id: number;
  email: string;
  name: string;
  disabled: boolean;
  created_at: number;
}

/** Fresh sign-in answer (`cms_auth::CmsSession`): profile + balance. */
export interface CmsSession {
  user: CmsUser;
  balance: number;
}

/** Stored-session snapshot (`cms_auth::CmsAccount`). */
export interface CmsAccount {
  user: CmsUser;
  balance: number;
}

/** `auth_status` answer: signed-out shells carry no account. */
export interface AuthStatus {
  loggedIn: boolean;
  account: CmsAccount | null;
}

/** True when the stored cms cookie was rejected → guide a re-login. */
export function isSessionExpiredError(error: unknown): boolean {
  return isApiError(error) && error.code === "SESSION_EXPIRED";
}

function requireDesktop(): void {
  if (!hasTauriRuntime()) {
    throw new ApiError(
      "NOT_IMPLEMENTED",
      "account sign-in needs the desktop shell",
      "launch the Tauri app to sign in",
    );
  }
}

export async function login(email: string, password: string): Promise<CmsSession> {
  requireDesktop();
  try {
    return await invoke<CmsSession>("auth_login", { input: { email, password } });
  } catch (error) {
    throw toApiError(error);
  }
}

export async function register(
  email: string,
  password: string,
  name: string,
): Promise<CmsUser> {
  requireDesktop();
  try {
    return await invoke<CmsUser>("auth_register", { input: { email, password, name } });
  } catch (error) {
    throw toApiError(error);
  }
}

export async function fetchAccount(): Promise<CmsAccount> {
  requireDesktop();
  try {
    return await invoke<CmsAccount>("auth_me");
  } catch (error) {
    throw toApiError(error);
  }
}

export async function fetchAuthStatus(): Promise<AuthStatus> {
  if (!hasTauriRuntime()) return { loggedIn: false, account: null };
  try {
    return await invoke<AuthStatus>("auth_status");
  } catch (error) {
    throw toApiError(error);
  }
}

export async function logout(): Promise<void> {
  requireDesktop();
  try {
    await invoke<void>("auth_logout");
  } catch (error) {
    throw toApiError(error);
  }
}
