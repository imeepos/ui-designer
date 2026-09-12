import { useEffect, useState } from "react";
import { Check, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { SizePreset } from "@/lib/api/types";
import { useStudio } from "@/state/studio";
import { ArtImage } from "@/components/art-image";
import { AnchorBadge } from "@/components/anchor-badge";
import { BriefField, CountPicker, QualityPicker } from "@/components/detail-forms";
import { JobPanel } from "@/components/job-panel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Modal } from "@/components/ui/modal";
import { Textarea } from "@/components/ui/textarea";
import { DEFAULT_QUALITY, type QualityLevel } from "@/lib/form-schema";
import { SIZE_PRESETS, validateCanvasSize } from "@/lib/size";
import { cn } from "@/lib/utils";

const PRESET_OPTIONS: SizePreset[] = ["web", "mobile", "desktop", "custom"];

/**
 * New-project wizard (single dialog, three sections):
 * 1) project info -> creates the record (stay in the wizard);
 * 2) generate the overview (board candidates + anchor pick), skippable;
 * 3) save -> enter the project workspace.
 */
export function CreateWizard({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const { state, createProject, generateBoard, pickAnchor, enterWorkspace } = useStudio();
  const project = state.project;

  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [name, setName] = useState("");
  const [preset, setPreset] = useState<SizePreset>("web");
  const [width, setWidth] = useState("1536");
  const [height, setHeight] = useState("1024");
  const [brandBrief, setBrandBrief] = useState("");
  const [sizeError, setSizeError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const [brandKeywords, setBrandKeywords] = useState("");
  const [colorDirection, setColorDirection] = useState("");
  const [fontMood, setFontMood] = useState("");
  const [radiusDensity, setRadiusDensity] = useState("");
  const [reference, setReference] = useState("");
  const [count, setCount] = useState(2);
  const [quality, setQuality] = useState<QualityLevel>(DEFAULT_QUALITY);

  useEffect(() => {
    if (!open) return;
    setStep(1);
    setName("");
    setPreset("web");
    setWidth("1536");
    setHeight("1024");
    setBrandBrief("");
    setSizeError(null);
    setCreating(false);
    setBrandKeywords("");
    setColorDirection("");
    setFontMood("");
    setRadiusDensity("");
    setReference("");
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

  const submitInfo = async () => {
    if (creating) return;
    const check = validateCanvasSize(preset, width, height);
    if (!check.ok) {
      setSizeError(t(`project.form.error.${check.code}`, check.params ?? {}));
      return;
    }
    setSizeError(null);
    setCreating(true);
    const detail = await createProject({
      name,
      size: check.size,
      brandBrief,
    });
    setCreating(false);
    if (!detail) return;
    setBrandKeywords(brandBrief);
    setStep(2);
  };

  const boardJob = state.job?.kind === "board" ? state.job : null;
  const anchor = project?.anchor ?? null;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("wizard.title")}
      testId="create-wizard"
      wide
    >
      <div className="flex flex-col gap-5">
        {/* Section 1: project info */}
        <section className="flex flex-col gap-3">
          <SectionTitle
            index={1}
            label={t("wizard.step.info")}
            state={step > 1 ? "done" : "current"}
          />
          {step === 1 ? (
            <div className="flex flex-col gap-3">
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="wizard-name">{t("project.form.name")}</Label>
                <Input
                  id="wizard-name"
                  data-testid="wizard-name-input"
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  placeholder={t("project.form.namePlaceholder")}
                  autoFocus
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label>{t("project.form.size")}</Label>
                <div
                  className="grid grid-cols-4 gap-1 rounded-md border p-1"
                  role="group"
                  aria-label={t("project.form.size")}
                >
                  {PRESET_OPTIONS.map((option) => (
                    <button
                      key={option}
                      type="button"
                      data-testid={`wizard-preset-${option}`}
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
                  <div className="flex items-center gap-2">
                    <Input
                      data-testid="wizard-width"
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
                      data-testid="wizard-height"
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
                ) : (
                  <p className="font-mono text-[11px] text-muted-foreground">
                    {preset === "web" ? "1536x1024" : preset === "mobile" ? "1024x1536" : "2560x1440"}
                  </p>
                )}
                <p className="text-[11px] text-muted-foreground">{t("project.form.sizeHint")}</p>
                {sizeError && (
                  <p data-testid="wizard-size-error" className="text-xs text-destructive">
                    {sizeError}
                  </p>
                )}
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="wizard-brief">{t("project.form.brandBrief")}</Label>
                <Textarea
                  id="wizard-brief"
                  data-testid="wizard-brief-input"
                  value={brandBrief}
                  onChange={(event) => setBrandBrief(event.target.value)}
                  placeholder={t("project.form.brandBriefPlaceholder")}
                  rows={2}
                />
              </div>
              <div>
                <Button
                  size="sm"
                  data-testid="wizard-create"
                  disabled={!name.trim() || creating}
                  onClick={() => void submitInfo()}
                >
                  {creating && <Loader2 aria-hidden="true" className="size-3.5 animate-spin" />}
                  {t("wizard.createAndNext")}
                </Button>
              </div>
            </div>
          ) : (
            <p className="text-xs text-muted-foreground">
              {name} · {project ? `${project.size.w}x${project.size.h}` : ""}
            </p>
          )}
        </section>

        {/* Section 2: generate the overview (board + anchor), skippable */}
        {step >= 2 && (
          <section className="flex flex-col gap-3 border-t pt-4">
            <SectionTitle
              index={2}
              label={t("wizard.step.overview")}
              state={anchor != null ? "done" : step === 2 ? "current" : "future"}
            />
            <div className="grid gap-3 md:grid-cols-2">
              <div className="flex flex-col gap-2.5">
                <BriefField
                  id="wizard-brand"
                  testId="wizard-brand"
                  label={t("form.board.brandKeywords")}
                  placeholder={t("form.board.brandPlaceholder")}
                  value={brandKeywords}
                  onChange={setBrandKeywords}
                  rows={2}
                />
                <BriefField
                  id="wizard-color"
                  testId="wizard-color"
                  label={t("form.board.colorDirection")}
                  placeholder={t("form.board.colorPlaceholder")}
                  value={colorDirection}
                  onChange={setColorDirection}
                  rows={2}
                />
                <BriefField
                  id="wizard-font"
                  testId="wizard-font"
                  label={t("form.board.fontMood")}
                  placeholder={t("form.board.fontPlaceholder")}
                  value={fontMood}
                  onChange={setFontMood}
                  rows={2}
                />
              </div>
              <div className="flex flex-col gap-2.5">
                <BriefField
                  id="wizard-radius"
                  testId="wizard-radius"
                  label={t("form.board.radiusDensity")}
                  placeholder={t("form.board.radiusPlaceholder")}
                  value={radiusDensity}
                  onChange={setRadiusDensity}
                  rows={2}
                />
                <BriefField
                  id="wizard-reference"
                  testId="wizard-reference"
                  label={t("form.board.reference")}
                  placeholder={t("form.board.referencePlaceholder")}
                  value={reference}
                  onChange={setReference}
                  rows={2}
                />
                <CountPicker value={count} onChange={setCount} testIdPrefix="wizard" />
                <QualityPicker value={quality} onChange={setQuality} testIdPrefix="wizard" />
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                data-testid="wizard-generate"
                disabled={!brandKeywords.trim() || state.job !== null || anchor != null}
                onClick={() =>
                  void generateBoard(
                    { brandKeywords, colorDirection, fontMood, radiusDensity, reference },
                    count,
                    quality,
                  )
                }
              >
                {project && project.boardCandidates.length > 0
                  ? t("form.board.regenerate")
                  : t("form.board.generate")}
              </Button>
              {anchor == null && (
                <Button
                  variant="ghost"
                  size="sm"
                  data-testid="wizard-skip"
                  onClick={() => setStep(3)}
                >
                  {t("wizard.skip")}
                </Button>
              )}
            </div>
            {boardJob && <JobPanel job={boardJob} />}
            {project && project.boardCandidates.length > 0 && (
              <div className="grid grid-cols-4 gap-2" data-testid="wizard-candidates">
                {project.boardCandidates.map((candidate) => {
                  const isAnchor = anchor?.candidateId === candidate.id;
                  return (
                    <button
                      key={candidate.id}
                      type="button"
                      data-testid={`wizard-candidate-${candidate.id}`}
                      aria-pressed={isAnchor}
                      onClick={() => void pickAnchor(candidate.id)}
                      className={cn(
                        "overflow-hidden rounded-md border transition-colors duration-150 ease-out",
                        isAnchor
                          ? "border-primary ring-1 ring-primary"
                          : "border-border hover:border-primary/60",
                      )}
                    >
                      <ArtImage
                        src={candidate.url}
                        filter={candidate.filter}
                        alt={t("gallery.board.candidateAlt", { id: candidate.id })}
                        className="aspect-square w-full"
                      />
                      {isAnchor && (
                        <span className="flex items-center justify-center gap-1 py-0.5 text-[11px] font-medium text-primary">
                          <Check className="size-3" />
                          {t("gallery.board.anchorBadge")}
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
            )}
            {anchor && (
              <div className="flex items-center gap-2">
                <AnchorBadge />
                <Button size="sm" data-testid="wizard-to-save" onClick={() => setStep(3)}>
                  {t("wizard.nextSave")}
                </Button>
              </div>
            )}
          </section>
        )}

        {/* Section 3: save -> enter the workspace */}
        {step >= 3 && (
          <section className="flex flex-col gap-2 border-t pt-4">
            <SectionTitle index={3} label={t("wizard.step.save")} state="current" />
            <p className="text-xs text-muted-foreground">{t("wizard.saveHint")}</p>
            <div>
              <Button
                size="sm"
                data-testid="wizard-save"
                onClick={() => {
                  enterWorkspace();
                  onClose();
                }}
              >
                {t("wizard.saveEnter")}
              </Button>
            </div>
          </section>
        )}
      </div>
    </Modal>
  );
}

/** THEME §5 stepper tri-state: done=primary check · current=accent dot + bold · future=muted outline. */
type SectionState = "done" | "current" | "future";

function SectionTitle({
  index,
  label,
  state,
}: {
  index: number;
  label: string;
  state: SectionState;
}) {
  return (
    <h3
      data-testid={`wizard-section-${index}`}
      data-section-state={state}
      className={cn(
        "flex items-center gap-2 text-sm font-medium text-foreground",
        state === "current" && "font-bold",
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          "flex size-5 shrink-0 items-center justify-center rounded-full font-mono text-[11px]",
          state === "done" && "border border-primary bg-primary text-primary-foreground",
          state === "current" && "border border-accent",
          state === "future" && "border border-border text-muted-foreground",
        )}
      >
        {state === "done" ? (
          <Check className="size-3" />
        ) : state === "current" ? (
          <span className="size-2 rounded-full bg-accent" />
        ) : (
          index
        )}
      </span>
      {label}
    </h3>
  );
}
