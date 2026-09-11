import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Modal } from "@/components/ui/modal";
import {
  fetchAccount,
  fetchAuthStatus,
  isSessionExpiredError,
  login,
  logout,
  register,
  type CmsAccount,
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
/**
 * Account view state machine: `checking` reads the stored cms session;
 * `signedIn` shows the account card; `signedOut` shows the form; `expired`
 * means the stored cookie was rejected → re-login guidance above the form.
 */
type AccountPhase = "checking" | "signedOut" | "signedIn" | "expired";

/**
 * Settings dialog: the top "Account" section signs in against the cms
 * service (session cookie + image key live only in the OS keychain); the
 * "Connection" section keeps the free /models probe.
 */
export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const { t } = useTranslation();
  const toast = useToast();

  /** Localized copy for a cms/auth error (inline form feedback). */
  const describeError = (error: unknown): string => {
    if (isApiError(error)) {
      return t([`errors.${error.code}.message`, "errors.UNKNOWN.message"], {
        detail: error.message,
        defaultValue: error.message,
      });
    }
    return error instanceof Error ? error.message : String(error);
  };

  const [phase, setPhase] = useState<AccountPhase>("checking");
  const [account, setAccount] = useState<CmsAccount | null>(null);
  const [mode, setMode] = useState<AccountMode>("login");
  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [busy, setBusy] = useState<BusyKind | null>(null);
  const [testOutcome, setTestOutcome] = useState<TestOutcome>(null);

  // (Re)load the sign-in state each time the dialog opens.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setPassword("");
    setEmail("");
    setName("");
    setFormError(null);
    setTestOutcome(null);
    setAccount(null);
    setPhase("checking");
    fetchAuthStatus()
      .then((status) => {
        if (cancelled) return;
        if (status.loggedIn && status.account) {
          setAccount(status.account);
          setPhase("signedIn");
        } else {
          setPhase("signedOut");
        }
      })
      .catch((error) => {
        if (cancelled) return;
        if (isSessionExpiredError(error)) {
          // Stored cookie rejected → re-login guidance, not a raw error.
          setPhase("expired");
        } else {
          setPhase("signedOut");
          toast.error(error);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [open, toast]);

  const applySession = (session: CmsAccount) => {
    setAccount(session);
    setPhase("signedIn");
    setPassword("");
    setEmail("");
    setName("");
  };

  const handleAuth = async (kind: Extract<AccountMode, "login" | "register">) => {
    // Client-side validation first: the cms contract has no username, so the
    // form is email (+name on register) + password.
    const trimmedEmail = email.trim();
    const trimmedName = name.trim();
    if (!trimmedEmail) {
      setFormError(t("settings.account.errorEmailRequired"));
      return;
    }
    if (kind === "register" && !trimmedName) {
      setFormError(t("settings.account.errorNameRequired"));
      return;
    }
    if (!password) {
      setFormError(t("settings.account.errorPasswordRequired"));
      return;
    }

    setBusy(kind);
    setFormError(null);
    try {
      if (kind === "login") {
        const session = await login(trimmedEmail, password);
        applySession(session);
      } else {
        // cms register creates no session — sign in right after with the
        // same credentials (this also mints the image key).
        await register(trimmedEmail, password, trimmedName);
        const session = await login(trimmedEmail, password);
        applySession(session);
      }
      toast.success(t(kind === "login" ? "settings.account.loginOk" : "settings.account.registerOk"));
    } catch (error) {
      if (isSessionExpiredError(error)) setPhase("expired");
      setFormError(describeError(error));
    } finally {
      setBusy(null);
    }
  };

  const handleLogout = async () => {
    setBusy("logout");
    try {
      await logout();
      setAccount(null);
      setPhase("signedOut");
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
      const fresh = await fetchAccount();
      setAccount(fresh);
      setPhase("signedIn");
      toast.success(t("settings.account.refreshOk"));
    } catch (error) {
      if (isSessionExpiredError(error)) {
        // Cookie died mid-session → back to the sign-in form with guidance.
        setAccount(null);
        setPhase("expired");
      }
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
      {mode === "register" && (
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="settings-name">{t("settings.account.name")}</Label>
          <Input
            id="settings-name"
            data-testid="settings-name"
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              setFormError(null);
            }}
            placeholder={t("settings.account.namePlaceholder")}
            autoComplete="off"
            spellCheck={false}
          />
        </div>
      )}
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

          {phase === "checking" && (
            <p className="text-xs text-muted-foreground" data-testid="settings-account-checking">
              {t("settings.account.statusChecking")}
            </p>
          )}

          {phase === "signedIn" && account && (
            <div
              className="flex flex-col gap-3 rounded-md border bg-muted/40 p-3.5"
              data-testid="settings-account-user"
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex min-w-0 flex-col">
                  <span className="truncate text-sm font-medium" data-testid="settings-account-name">
                    {account.user.name}
                  </span>
                  <span className="truncate text-xs text-muted-foreground" data-testid="settings-account-email">
                    {account.user.email}
                  </span>
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
                  <p className="text-3xl font-semibold tabular-nums leading-none" data-testid="settings-account-balance">
                    {account.balance}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">{t("settings.account.balance")}</p>
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
                  date: new Date(account.user.created_at * 1000).toLocaleDateString(),
                })}
              </p>
            </div>
          )}

          {(phase === "signedOut" || phase === "expired") && (
            <div className="flex flex-col gap-2.5" data-testid="settings-account-form">
              {phase === "expired" && (
                <p
                  className="rounded-md border border-destructive/40 bg-destructive/5 px-2.5 py-2 text-xs text-destructive"
                  data-testid="settings-session-expired"
                  role="alert"
                >
                  {t("settings.account.sessionExpired")}
                </p>
              )}

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
                  disabled={busy !== null}
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
