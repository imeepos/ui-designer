import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Modal } from "@/components/ui/modal";
import {
  clearApiKey,
  getCredentialStatus,
  saveApiKey,
  saveConfig,
  testConnection,
  type CredentialKeySource,
  type CredentialStatus,
} from "@/lib/api/credentials";
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

type BusyKind = "save" | "clear" | "test";

const DEFAULT_BASE_PLACEHOLDER = "https://api.openai.com";
const DEFAULT_MODEL_PLACEHOLDER = "gpt-image-2";

/** Model ids shown in the datalist: image-family first, all models as fallback. */
function toModelOptions(models: string[]): string[] {
  const imageFamily = models.filter((model) => model.toLowerCase().includes("image"));
  return imageFamily.length > 0 ? imageFamily : models;
}

/**
 * Settings dialog: non-sensitive Base URL + model name live in
 * ~/Rudder/config.json; the API key lives ONLY in the OS keychain (masked
 * input, tail-4 display after save). env vars keep priority over both
 * (`OPENAI_BASE_URL` / `OPENAI_MODEL` / `OPENAI_API_KEY`).
 */
export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const { t } = useTranslation();
  const toast = useToast();

  const [status, setStatus] = useState<CredentialStatus | null>(null);
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [modelOptions, setModelOptions] = useState<string[]>([]);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [busy, setBusy] = useState<BusyKind | null>(null);
  const [testOutcome, setTestOutcome] = useState<TestOutcome>(null);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setApiKeyInput("");
    setTestOutcome(null);
    setModelOptions([]);
    getCredentialStatus()
      .then((next) => {
        if (cancelled) return;
        setStatus(next);
        setBaseUrl(next.baseUrl);
        setModel(next.model);
      })
      .catch((error) => {
        if (!cancelled) toast.error(error);
      });
    return () => {
      cancelled = true;
    };
  }, [open, toast]);

  const sourceLabel = (source: CredentialKeySource) => {
    if (source === "env") return t("settings.sourceEnv");
    if (source === "keychain") return t("settings.sourceKeychain");
    return t("settings.sourceNone");
  };

  const describeTestError = (error: unknown): string => {
    if (isApiError(error)) {
      return t([`errors.${error.code}.message`, "errors.UNKNOWN.message"], {
        defaultValue: error.message,
      });
    }
    return error instanceof Error ? error.message : String(error);
  };

  const handleSave = async () => {
    setBusy("save");
    try {
      await saveConfig({ baseUrl, model });
      const trimmedKey = apiKeyInput.trim();
      const next = trimmedKey ? await saveApiKey(trimmedKey) : await getCredentialStatus();
      setStatus(next);
      setBaseUrl(next.baseUrl);
      setModel(next.model);
      setApiKeyInput("");
      setTestOutcome(null);
      toast.success(t("settings.saveOk"));
    } catch (error) {
      toast.error(error);
    } finally {
      setBusy(null);
    }
  };

  const handleClear = async () => {
    setBusy("clear");
    try {
      const next = await clearApiKey();
      setStatus(next);
      setApiKeyInput("");
      setTestOutcome(null);
      toast.success(t("settings.cleared"));
    } catch (error) {
      toast.error(error);
    } finally {
      setBusy(null);
    }
  };

  const handleTest = async () => {
    setBusy("test");
    try {
      const result = await testConnection({
        baseUrl: baseUrl.trim() || undefined,
        apiKey: apiKeyInput.trim() || undefined,
      });
      setModelOptions(toModelOptions(result.models));
      setTestOutcome({ ok: true, modelCount: result.modelCount });
    } catch (error) {
      setTestOutcome({ ok: false, message: describeTestError(error) });
    } finally {
      setBusy(null);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("settings.title")}
      testId="settings-dialog"
      footer={
        <>
          <Button
            variant="ghost"
            size="sm"
            data-testid="settings-clear"
            disabled={busy !== null || status?.keySource !== "keychain"}
            onClick={() => void handleClear()}
          >
            {t("settings.clear")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            data-testid="settings-test"
            disabled={busy !== null}
            onClick={() => void handleTest()}
          >
            {busy === "test" ? t("settings.testing") : t("settings.test")}
          </Button>
          <Button
            size="sm"
            data-testid="settings-save"
            disabled={busy !== null}
            onClick={() => void handleSave()}
          >
            {busy === "save" ? t("settings.saving") : t("settings.save")}
          </Button>
        </>
      }
    >
      <form
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault();
          void handleSave();
        }}
      >
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="settings-base-url">{t("settings.baseUrl")}</Label>
          <Input
            id="settings-base-url"
            data-testid="settings-base-url"
            value={baseUrl}
            onChange={(event) => {
              setBaseUrl(event.target.value);
              setTestOutcome(null);
            }}
            placeholder={DEFAULT_BASE_PLACEHOLDER}
            className="font-mono"
            autoComplete="off"
            spellCheck={false}
          />
          <p className="text-[11px] text-muted-foreground">{t("settings.baseUrlHint")}</p>
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="settings-api-key">{t("settings.apiKey")}</Label>
          <Input
            id="settings-api-key"
            data-testid="settings-api-key"
            type="password"
            value={apiKeyInput}
            onChange={(event) => {
              setApiKeyInput(event.target.value);
              setTestOutcome(null);
            }}
            placeholder={t("settings.apiKeyPlaceholder")}
            autoComplete="off"
          />
          {status?.keySource === "keychain" && status.keyTail && (
            <p className="text-[11px] text-muted-foreground" data-testid="settings-key-status">
              {t("settings.apiKeyStored", { tail: status.keyTail })}
            </p>
          )}
          {status?.keySource === "env" && status.keyTail && (
            <p className="text-[11px] text-muted-foreground" data-testid="settings-key-status">
              {t("settings.apiKeyEnv", { tail: status.keyTail })}
            </p>
          )}
          <p className="text-[11px] text-muted-foreground" data-testid="settings-key-source">
            {t("settings.keySourceValue", {
              source: status ? sourceLabel(status.keySource) : "…",
            })}
          </p>
          <p className="text-[11px] text-muted-foreground">{t("settings.priorityHint")}</p>
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="settings-model">{t("settings.model")}</Label>
          <Input
            id="settings-model"
            data-testid="settings-model"
            value={model}
            onChange={(event) => {
              setModel(event.target.value);
              setTestOutcome(null);
            }}
            placeholder={DEFAULT_MODEL_PLACEHOLDER}
            list="settings-model-options"
            className="font-mono"
            autoComplete="off"
            spellCheck={false}
          />
          <datalist id="settings-model-options" data-testid="settings-model-options">
            {modelOptions.map((option) => (
              <option key={option} value={option} />
            ))}
          </datalist>
          <p className="text-[11px] text-muted-foreground">{t("settings.modelHint")}</p>
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
              ? t("settings.testOk", { count: testOutcome.modelCount })
              : `${t("settings.testFailed")} — ${testOutcome.message}`}
          </p>
        )}
      </form>
    </Modal>
  );
}
