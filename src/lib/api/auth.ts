import { invoke } from "@tauri-apps/api/core";

import { hasTauriRuntime } from "@/lib/api";
import { toApiError } from "@/lib/api/tauri-api";
import { ApiError } from "@/lib/api/types";

/**
 * Server account API (Settings dialog "Account" section): sign in / register
 * against the configured rudder-server through dedicated Tauri commands.
 *
 * The session token is persisted ONLY in the OS keychain on the Rust side;
 * the value returned here stays in memory and must never be logged, rendered
 * or stored by the frontend. In the browser mock preview these calls degrade
 * gracefully: the status read reports "no session", mutations reject
 * NOT_IMPLEMENTED.
 */

export interface RudderUser {
  id: string;
  username: string;
  email: string | null;
  /** `user` | `admin` (admin accounts generate for free). */
  role: "user" | "admin";
  status: string;
  /** Pay-per-image balance (default 10 credits per image). */
  credits: number;
  createdAt: string;
}

export interface RudderSession {
  token: string;
  user: RudderUser;
}

export interface SessionStatus {
  hasToken: boolean;
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

export async function login(username: string, password: string): Promise<RudderSession> {
  requireDesktop();
  try {
    return await invoke<RudderSession>("auth_login", { input: { username, password } });
  } catch (error) {
    throw toApiError(error);
  }
}

export async function register(
  username: string,
  password: string,
  email?: string,
): Promise<RudderSession> {
  requireDesktop();
  try {
    return await invoke<RudderSession>("auth_register", {
      input: { username, password, email: email ?? null },
    });
  } catch (error) {
    throw toApiError(error);
  }
}

export async function fetchMe(): Promise<RudderUser> {
  requireDesktop();
  try {
    return await invoke<RudderUser>("auth_me");
  } catch (error) {
    throw toApiError(error);
  }
}

export async function fetchSessionStatus(): Promise<SessionStatus> {
  if (!hasTauriRuntime()) return { hasToken: false };
  try {
    return await invoke<SessionStatus>("get_session_status");
  } catch (error) {
    throw toApiError(error);
  }
}

export async function logout(): Promise<void> {
  requireDesktop();
  try {
    await invoke<void>("clear_session_token");
  } catch (error) {
    throw toApiError(error);
  }
}
