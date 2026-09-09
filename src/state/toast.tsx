import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";

import { isApiError } from "@/lib/api/types";
import { requestOpenSettings } from "@/lib/events";
import { ToastViewport } from "@/components/ui/toast";

export type ToastVariant = "success" | "error" | "info";

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface ToastItem {
  id: number;
  variant: ToastVariant;
  message: string;
  hint?: string;
  /** Error code badge (ApiError.code), rendered in mono. */
  code?: string;
  /** Optional inline action (e.g. NO_CREDENTIALS → open Settings). */
  action?: ToastAction;
}

export interface ToastApi {
  success: (message: string) => void;
  info: (message: string) => void;
  /** Maps unknown errors / ApiError into a localized code+message+hint toast. */
  error: (error: unknown) => void;
}

const ToastContext = createContext<ToastApi | null>(null);

const SUCCESS_TIMEOUT_MS = 4_000;
const INFO_TIMEOUT_MS = 4_000;
const ERROR_TIMEOUT_MS = 9_000;

export function ToastProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const [items, setItems] = useState<ToastItem[]>([]);
  const nextId = useRef(0);

  const dismiss = useCallback((id: number) => {
    setItems((prev) => prev.filter((item) => item.id !== id));
  }, []);

  const push = useCallback(
    (item: Omit<ToastItem, "id">, timeoutMs: number) => {
      nextId.current += 1;
      const id = nextId.current;
      setItems((prev) => [...prev.slice(-3), { ...item, id }]);
      window.setTimeout(() => dismiss(id), timeoutMs);
    },
    [dismiss],
  );

  const api = useMemo<ToastApi>(
    () => ({
      success: (message) => {
        push({ variant: "success", message }, SUCCESS_TIMEOUT_MS);
      },
      info: (message) => {
        push({ variant: "info", message }, INFO_TIMEOUT_MS);
      },
      error: (error) => {
        if (isApiError(error)) {
          if (error.code === "CANCELLED") {
            push(
              { variant: "info", message: t("toast.cancelled") },
              INFO_TIMEOUT_MS,
            );
            return;
          }
          // VALIDATION_ERROR carries the core's specific reason in
          // `error.message` (e.g. "invalid component name ..."); surface it
          // via the {{detail}} placeholder (UI-REVIEW P2), overriding any
          // adapter-provided params of the same name.
          const params = { ...error.params, detail: error.message };
          const message = t([`errors.${error.code}.message`, "errors.UNKNOWN.message"], {
            ...params,
            defaultValue: error.message,
          });
          const hint = error.hint
            ? t([`errors.${error.code}.hint`, "errors.UNKNOWN.hint"], {
                ...params,
                defaultValue: error.hint,
              })
            : undefined;
          // NO_CREDENTIALS guides the user straight into the Settings dialog
          // (the keychain can only be filled there or via the CLI).
          const action =
            error.code === "NO_CREDENTIALS"
              ? { label: t("toast.openSettings"), run: requestOpenSettings }
              : undefined;
          push(
            { variant: "error", message, hint, code: error.code, action },
            ERROR_TIMEOUT_MS,
          );
          return;
        }
        push(
          {
            variant: "error",
            message: t("errors.UNKNOWN.message"),
            hint: error instanceof Error ? error.message : undefined,
            code: "UNKNOWN",
          },
          ERROR_TIMEOUT_MS,
        );
      },
    }),
    [push, t],
  );

  return (
    <ToastContext.Provider value={api}>
      {children}
      <ToastViewport items={items} onDismiss={dismiss} />
    </ToastContext.Provider>
  );
}

export function useToast(): ToastApi {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    throw new Error("useToast must be used within ToastProvider");
  }
  return ctx;
}
