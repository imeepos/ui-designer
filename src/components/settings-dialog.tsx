import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Modal } from "@/components/ui/modal";
import {
  fetchMe,
  fetchSessionStatus,
  login,
  logout,
  register,
  type RudderUser,
} from "@/lib/api/auth";
import { testConnection } from "@/lib/api/credentials";
import { isApiError } from "@/lib/api/types";
import { cn } from "@/lib/utils";
import { useToast } from "@/state/toast";

type SettingsDialogProps = {
  open: boolean;
  onClose: () => void;
};

type TestOutcome =
  | { ok: true; modelCount: number }
  | { ok: false; message: string }
  | null;

type AccountMode = "login" | "register";
type BusyKind = "login" | "register" | "logout" | "refresh" | "test";
type SessionState = { hasToken: boolean } | null;

/**
 * Settings dialog: the top "Account" section signs in against the configured
 * rudder-server (the session token lives only in the OS keychain); the
 * "Connection" section keeps the free /models probe. The legacy manual
 * baseUrl / API key / model inputs are gone — the desktop shell is
 * zero-config.
 */
export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const { t } = useTranslation();
  const toast = useToast();

  /** Localized copy for a server/auth error (inline form feedback). */
  const describeError = (error: unknown): string => {
    if (isApiError(error)) {
      return t([`errors.${error.code}.message`, "errors.UNKNOWN.message"], {
        detail: error.message,
        defaultValue: error.message,
      });
    }
    return error instanceof Error ? error.message : String(error);
  };

  const [session, setSession] = useState<SessionState>(null);
  const [user, setUser] = useState<RudderUser | null>(null);
  const [mode, setMode] = useState<AccountMode>("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [email, setEmail] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [busy, setBusy] = useState<BusyKind | null>(null);
  const [testOutcome, setTestOutcome] = useState<TestOutcome>(null);

  // (Re)load the session state each time the dialog opens.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setPassword("");
    setEmail("");
    setFormError(null);
    setTestOutcome(null);
    setUser(null);
    setSession(null);
    fetchSessionStatus()
      .then(async (status) => {
        if (cancelled) return;
        setSession(status);
        if (status.hasToken) {
          try {
            const me = await fetchMe();
            if (!cancelled) setUser(me);
          } catch (error) {
            if (!cancelled) {
              // Stale token (expired/revoked) → back to the sign-in form.
              setSession({ hasToken: false });
              toast.error(error);
            }
          }
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setSession({ hasToken: false });
          toast.error(error);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [open, toast]);

  const handleAuth = async (kind: Extract<AccountMode, "login" | "register">) => {
    setBusy(kind);
    setFormError(null);
    try {
      const next =
        kind === "login"
          ? await login(username.trim(), password)
          : await register(username.trim(), password, email.trim() || undefined);
      setSession({ hasToken: true });
      setUser(next.user);
      setPassword("");
      setEmail("");
      toast.success(t(kind === "login" ? "settings.account.loginOk" : "settings.account.registerOk"));
    } catch (error) {
      setFormError(describeError(error));
    } finally {
      setBusy(null);
    }
  };

  const handleLogout = async () => {
    setBusy("logout");
    try {
      await logout();
      setSession({ hasToken: false });
      setUser(null);
      setMode("login");
      toast.success(t("settings.account.logoutOk"));
    } catch (error) {
      toast.error(error);
    } finally {
      setBusy(null);
    }
  };

  const handleRefresh = async () => {
    setBusy("refresh");
    try {
      const me = await fetchMe();
      setUser(me);
      toast.success(t("settings.account.refreshOk"));
    } catch (error) {
      toast.error(error);
    } finally {
      setBusy(null);
    }
  };

  const handleTest = async () => {
    setBusy("test");
    try {
      const result = await testConnection();
      setTestOutcome({ ok: true, modelCount: result.modelCount });
    } catch (error) {
      setTestOutcome({ ok: false, message: describeError(error) });
    } finally {
      setBusy(null);
    }
  };

  const authInputs = (
    <>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="settings-username">{t("settings.account.username")}</Label>
        <Input
          id="settings-username"
          data-testid="settings-username"
          value={username}
          onChange={(event) => {
            setUsername(event.target.value);
            setFormError(null);
          }}
          placeholder={t("settings.account.usernamePlaceholder")}
          autoComplete="off"
          spellCheck={false}
        />
      </div>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="settings-password">{t("settings.account.password")}</Label>
        <Input
          id="settings-password"
          data-testid="settings-password"
          type="password"
          value={password}
          onChange={(event) => {
            setPassword(event.target.value);
            setFormError(null);
          }}
          placeholder={t("settings.account.passwordPlaceholder")}
          autoComplete="off"
        />
      </div>
    </>
  );

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("settings.title")}
      testId="settings-dialog"
    >
      <div className="flex flex-col gap-5">
        {/* ------------------------------------------------ account (top) */}
        <section className="flex flex-col gap-2.5" data-testid="settings-account">
          <h3 className="text-sm font-medium text-foreground">{t("settings.account.title")}</h3>

          {session === null && (
            <p className="text-xs text-muted-foreground" data-testid="settings-account-checking">
              {t("settings.account.statusChecking")}
            </p>
          )}

          {session !== null && session.hasToken && user && (
            <div
              className="flex flex-col gap-3 rounded-md border bg-muted/40 p-3.5"
              data-testid="settings-account-user"
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex min-w-0 items-center gap-2">
                  <span className="truncate text-sm font-medium" data-testid="settings-account-username">
                    {user.username}
                  </span>
                  <Badge variant={user.role === "admin" ? "default" : "secondary"} data-testid="settings-account-role">
                    {user.role === "admin"
                      ? t("settings.account.roleAdmin")
                      : t("settings.account.roleUser")}
                  </Badge>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  data-testid="settings-logout"
                  disabled={busy !== null}
                  onClick={() => void handleLogout()}
                >
                  {t("settings.account.logout")}
                </Button>
              </div>

              <div className="flex items-end justify-between gap-2">
                <div>
                  <p className="text-3xl font-semibold tabular-nums leading-none" data-testid="settings-account-credits">
                    {user.credits}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">{t("settings.account.credits")}</p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  data-testid="settings-refresh"
                  disabled={busy !== null}
                  onClick={() => void handleRefresh()}
                >
                  {busy === "refresh" ? t("settings.account.refreshBusy") : t("settings.account.refresh")}
                </Button>
              </div>

              <p className="text-[11px] text-muted-foreground">
                {t("settings.account.memberSince", {
                  date: new Date(user.createdAt).toLocaleDateString(),
                })}
              </p>
            </div>
          )}

          {session !== null && !session.hasToken && (
            <div className="flex flex-col gap-2.5" data-testid="settings-account-form">
              {/* Segmented login/register toggle (no Tabs dependency). */}
              <div className="inline-flex w-fit rounded-md border p-0.5" role="tablist">
                {(["login", "register"] as const).map((tab) => (
                  <Button
                    key={tab}
                    variant={mode === tab ? "secondary" : "ghost"}
                    size="sm"
                    role="tab"
                    aria-selected={mode === tab}
                    data-testid={`settings-tab-${tab}`}
                    className={cn(mode === tab && "shadow-none")}
                    disabled={busy !== null}
                    onClick={() => {
                      setMode(tab);
                      setFormError(null);
                    }}
                  >
                    {t(tab === "login" ? "settings.account.tabLogin" : "settings.account.tabRegister")}
                  </Button>
                ))}
              </div>

              <form
                className="flex flex-col gap-2.5"
                onSubmit={(event) => {
                  event.preventDefault();
                  if (busy === null) void handleAuth(mode);
                }}
              >
                {authInputs}
                {mode === "register" && (
                  <div className="flex flex-col gap-1.5">
                    <Label htmlFor="settings-email">{t("settings.account.email")}</Label>
                    <Input
                      id="settings-email"
                      data-testid="settings-email"
                      type="email"
                      value={email}
                      onChange={(event) => {
                        setEmail(event.target.value);
                        setFormError(null);
                      }}
                      placeholder={t("settings.account.emailPlaceholder")}
                      autoComplete="off"
                      spellCheck={false}
                    />
                  </div>
                )}

                {formError && (
                  <p className="text-xs text-destructive" data-testid="settings-account-error" role="alert">
                    {formError}
                  </p>
                )}

                <Button
                  type="submit"
                  size="sm"
                  className="w-fit"
                  data-testid={mode === "login" ? "settings-login" : "settings-register"}
                  disabled={busy !== null || !username.trim() || password.length === 0}
                >
                  {busy === "login"
                    ? t("settings.account.loginBusy")
                    : busy === "register"
                      ? t("settings.account.registerBusy")
                      : mode === "login"
                        ? t("settings.account.login")
                        : t("settings.account.register")}
                </Button>
              </form>
            </div>
          )}
        </section>

        {/* -------------------------------------------- connection (kept) */}
        <section className="flex flex-col gap-2.5" data-testid="settings-connection">
          <div className="flex items-center justify-between gap-2">
            <h3 className="text-sm font-medium text-foreground">
              {t("settings.connection.title")}
            </h3>
            <Button
              variant="outline"
              size="sm"
              data-testid="settings-test"
              disabled={busy !== null}
              onClick={() => void handleTest()}
            >
              {busy === "test" ? t("settings.connection.testing") : t("settings.connection.test")}
            </Button>
          </div>

          {testOutcome && (
            <p
              data-testid="settings-test-result"
              className={cn(
                "rounded-md border px-2.5 py-2 text-xs leading-snug",
                testOutcome.ok
                  ? "border-primary/30 bg-primary/5 text-primary"
                  : "border-destructive/40 bg-destructive/5 text-destructive",
              )}
            >
              {testOutcome.ok
                ? t("settings.connection.testOk", { count: testOutcome.modelCount })
                : `${t("settings.connection.testFailed")} — ${testOutcome.message}`}
            </p>
          )}
        </section>
      </div>
    </Modal>
  );
}
