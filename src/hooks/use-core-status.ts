import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type CoreStatus =
  | { state: "checking" }
  | { state: "ok"; version: string }
  | { state: "browser" };

function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Probe the Tauri `ping` command; degrades gracefully in browser preview */
export function useCoreStatus(): CoreStatus {
  const [status, setStatus] = useState<CoreStatus>({ state: "checking" });

  useEffect(() => {
    if (!hasTauriRuntime()) {
      setStatus({ state: "browser" });
      return;
    }
    let cancelled = false;
    invoke<{ message: string; version: string }>("ping")
      .then((reply) => {
        if (cancelled) return;
        setStatus(
          reply.message === "pong"
            ? { state: "ok", version: reply.version }
            : { state: "browser" },
        );
      })
      .catch(() => {
        if (!cancelled) setStatus({ state: "browser" });
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return status;
}
