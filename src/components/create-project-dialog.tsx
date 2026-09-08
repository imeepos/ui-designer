import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Modal } from "@/components/ui/modal";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { SIZE_PRESETS, validateCanvasSize } from "@/lib/size";
import type { SizePreset } from "@/lib/api/types";

type CreateProjectDialogProps = {
  open: boolean;
  onClose: () => void;
};

const PRESET_OPTIONS: SizePreset[] = ["web", "mobile", "desktop", "custom"];

/** Step 1: name + canvas size preset (custom validated) + brand brief. */
export function CreateProjectDialog({ open, onClose }: CreateProjectDialogProps) {
  const { t } = useTranslation();
  const { createProject } = useStudio();

  const [name, setName] = useState("");
  const [preset, setPreset] = useState<SizePreset>("web");
  const [width, setWidth] = useState("1536");
  const [height, setHeight] = useState("1024");
  const [brandBrief, setBrandBrief] = useState("");
  const [sizeError, setSizeError] = useState<{ code: string; params?: Record<string, string | number> } | null>(null);

  useEffect(() => {
    if (!open) return;
    setName("");
    setPreset("web");
    setWidth("1536");
    setHeight("1024");
    setBrandBrief("");
    setSizeError(null);
  }, [open]);

  const selectPreset = (next: SizePreset) => {
    setPreset(next);
    setSizeError(null);
    if (next !== "custom") {
      const fixed = SIZE_PRESETS[next];
      setWidth(String(fixed.w));
      setHeight(String(fixed.h));
    }
  };

  const submit = () => {
    const check = validateCanvasSize(preset, width, height);
    if (!check.ok) {
      setSizeError({ code: check.code, params: check.params });
      return;
    }
    setSizeError(null);
    void createProject({
      name,
      size: check.size,
      brandBrief,
    });
    onClose();
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("project.form.title")}
      testId="create-project-dialog"
      footer={
        <>
          <Button variant="outline" size="sm" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button size="sm" data-testid="create-project-submit" onClick={submit}>
            {t("project.form.submit")}
          </Button>
        </>
      }
    >
      <form
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="project-name">{t("project.form.name")}</Label>
          <Input
            id="project-name"
            data-testid="project-name-input"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder={t("project.form.namePlaceholder")}
            autoFocus
            required
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <Label>{t("project.form.size")}</Label>
          <div className="grid grid-cols-4 gap-1 rounded-md border p-1" role="group" aria-label={t("project.form.size")}>
            {PRESET_OPTIONS.map((option) => (
              <button
                key={option}
                type="button"
                data-testid={`size-preset-${option}`}
                aria-pressed={preset === option}
                onClick={() => selectPreset(option)}
                className={cn(
                  "rounded-sm px-2 py-1 text-xs transition-colors duration-150 ease-out",
                  preset === option
                    ? "bg-primary font-medium text-primary-foreground"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                {t(`project.form.preset.${option}`)}
              </button>
            ))}
          </div>
          {preset === "custom" ? (
            <div className="flex flex-col gap-1.5">
              <div className="flex items-center gap-2">
                <Input
                  data-testid="custom-width"
                  inputMode="numeric"
                  value={width}
                  onChange={(event) => {
                    setWidth(event.target.value);
                    setSizeError(null);
                  }}
                  placeholder="1536"
                  aria-label={t("project.form.width")}
                  className="font-mono"
                />
                <span className="text-xs text-muted-foreground">×</span>
                <Input
                  data-testid="custom-height"
                  inputMode="numeric"
                  value={height}
                  onChange={(event) => {
                    setHeight(event.target.value);
                    setSizeError(null);
                  }}
                  placeholder="1024"
                  aria-label={t("project.form.height")}
                  className="font-mono"
                />
              </div>
              <p className="text-[11px] text-muted-foreground">{t("project.form.sizeHint")}</p>
              {sizeError && (
                <p data-testid="size-error" className="text-xs text-destructive">
                  {t(`project.form.error.${sizeError.code}`, sizeError.params ?? {})}
                </p>
              )}
            </div>
          ) : (
            <p className="font-mono text-[11px] text-muted-foreground">
              {preset === "web" ? "1536x1024" : preset === "mobile" ? "1024x1536" : "2560x1440"}
            </p>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="project-brief">{t("project.form.brandBrief")}</Label>
          <Textarea
            id="project-brief"
            data-testid="project-brief-input"
            value={brandBrief}
            onChange={(event) => setBrandBrief(event.target.value)}
            placeholder={t("project.form.brandBriefPlaceholder")}
            rows={3}
          />
          <p className="text-[11px] text-muted-foreground">{t("project.form.brandBriefHint")}</p>
        </div>
      </form>
    </Modal>
  );
}
