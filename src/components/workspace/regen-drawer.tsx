import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { X as XIcon } from "lucide-react";

import type { BoardBrief } from "@/lib/api/types";
import { useStudio } from "@/state/studio";
import { BriefField, CountPicker, QualityPicker } from "@/components/detail-forms";
import { JobPanel } from "@/components/job-panel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { DEFAULT_QUALITY, type QualityLevel } from "@/lib/form-schema";

export type RegenMode = "board" | { kind: "page"; slug: string } | { kind: "component"; name: string };

/**
 * Left drawer for regeneration: edit brief / count / quality, then trigger.
 * An amended brief is persisted via the update ops BEFORE generating so both
 * Mock and Tauri behave the same (core prompt engine reads the stored brief).
 */
export function RegenDrawer({ mode, onClose }: { mode: RegenMode; onClose: () => void }) {
  const { t } = useTranslation();
  const { state, generateBoard, updatePageBrief, generatePage, updateComponentBrief, generateComponent } =
    useStudio();
  const project = state.project;

  const [brandKeywords, setBrandKeywords] = useState("");
  const [colorDirection, setColorDirection] = useState("");
  const [fontMood, setFontMood] = useState("");
  const [radiusDensity, setRadiusDensity] = useState("");
  const [reference, setReference] = useState("");
  const [brief, setBrief] = useState("");
  const [storedBrief, setStoredBrief] = useState("");
  const [count, setCount] = useState(2);
  const [quality, setQuality] = useState<QualityLevel>(DEFAULT_QUALITY);
  const [savingBrief, setSavingBrief] = useState(false);

  const kind = typeof mode === "string" ? mode : mode.kind;
  const job =
    state.job?.kind === kind
      ? state.job
      : null;

  useEffect(() => {
    if (!project) return;
    if (mode === "board") {
      setBrandKeywords(project.brandBrief);
      setColorDirection("");
      setFontMood("");
      setRadiusDensity("");
      setReference("");
    } else if (mode.kind === "page") {
      const page = project.pages.find((item) => item.slug === mode.slug);
      setBrief(page?.brief ?? "");
      setStoredBrief(page?.brief ?? "");
    } else {
      const component = project.components.find((item) => item.name === mode.name);
      setBrief(component?.brief ?? "");
      setStoredBrief(component?.brief ?? "");
    }
    // Re-seed the form whenever the drawer target changes.
  }, [mode, project]);

  if (!project) return null;

  const submit = async () => {
    if (mode === "board") {
      const payload: BoardBrief = { brandKeywords, colorDirection, fontMood, radiusDensity, reference };
      await generateBoard(payload, count, quality);
      return;
    }
    if (!brief.trim()) return;
    setSavingBrief(true);
    const persisted =
      mode.kind === "page"
        ? await updatePageBrief(mode.slug, brief.trim())
        : await updateComponentBrief(mode.name, brief.trim());
    setSavingBrief(false);
    if (!persisted) return;
    if (mode.kind === "page") {
      await generatePage(mode.slug, count, quality);
    } else {
      await generateComponent(mode.name, count, quality);
    }
  };

  const briefDirty = mode !== "board" && brief.trim() !== storedBrief;
  const running = job !== null || savingBrief;

  return (
    <div className="fixed inset-0 z-40" data-testid="regen-drawer" role="dialog" aria-modal="true">
      <button
        type="button"
        aria-label={t("common.close")}
        tabIndex={-1}
        onClick={onClose}
        className="absolute inset-0 bg-foreground/25"
      />
      <aside className="absolute inset-y-0 left-0 flex w-80 flex-col border-r bg-card shadow-[0_8px_24px_rgba(2,8,23,0.08)]">
        <div className="flex h-11 shrink-0 items-center justify-between border-b px-4">
          <h2 className="text-sm font-semibold">
            {mode === "board"
              ? t("drawer.titleBoard")
              : mode.kind === "page"
                ? t("drawer.titlePage", { name: mode.slug })
                : t("drawer.titleComponent", { name: mode.name })}
          </h2>
          <Button variant="ghost" size="icon" aria-label={t("common.close")} data-testid="drawer-close" onClick={onClose}>
            <XIcon className="size-4" />
          </Button>
        </div>

        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          {mode === "board" ? (
            <>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="drawer-brand">{t("form.board.brandKeywords")}</Label>
                <Input
                  id="drawer-brand"
                  data-testid="drawer-brand"
                  value={brandKeywords}
                  maxLength={200}
                  onChange={(event) => setBrandKeywords(event.target.value)}
                />
              </div>
              <BriefField
                id="drawer-color"
                testId="drawer-color"
                label={t("form.board.colorDirection")}
                placeholder={t("form.board.colorPlaceholder")}
                value={colorDirection}
                onChange={setColorDirection}
                rows={2}
              />
              <BriefField
                id="drawer-font"
                testId="drawer-font"
                label={t("form.board.fontMood")}
                placeholder={t("form.board.fontPlaceholder")}
                value={fontMood}
                onChange={setFontMood}
                rows={2}
              />
              <BriefField
                id="drawer-radius"
                testId="drawer-radius"
                label={t("form.board.radiusDensity")}
                placeholder={t("form.board.radiusPlaceholder")}
                value={radiusDensity}
                onChange={setRadiusDensity}
                rows={2}
              />
              <BriefField
                id="drawer-reference"
                testId="drawer-reference"
                label={t("form.board.reference")}
                placeholder={t("form.board.referencePlaceholder")}
                value={reference}
                onChange={setReference}
                rows={2}
              />
            </>
          ) : (
            <div className="flex flex-col gap-1.5">
              <div className="flex items-baseline justify-between gap-2">
                <Label htmlFor="drawer-brief">{t("drawer.brief")}</Label>
                {briefDirty && (
                  <span className="text-[11px] text-muted-foreground">{t("drawer.briefDirty")}</span>
                )}
              </div>
              <Textarea
                id="drawer-brief"
                data-testid="drawer-brief"
                value={brief}
                onChange={(event) => setBrief(event.target.value)}
                rows={5}
              />
            </div>
          )}

          <CountPicker value={count} onChange={setCount} testIdPrefix="drawer" />
          <QualityPicker value={quality} onChange={setQuality} testIdPrefix="drawer" />

          {job ? (
            <JobPanel job={job} />
          ) : (
            <Button
              size="sm"
              data-testid="drawer-generate"
              disabled={running || (mode !== "board" && !brief.trim()) || (mode === "board" && !brandKeywords.trim())}
              onClick={() => void submit()}
            >
              {mode === "board" ? t("drawer.generateBoard") : t("drawer.generate")}
            </Button>
          )}
          <p className="text-[11px] leading-relaxed text-muted-foreground">{t("drawer.hint")}</p>
        </div>
      </aside>
    </div>
  );
}
