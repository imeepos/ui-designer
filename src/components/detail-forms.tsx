import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import type { ComponentType } from "@/lib/api/types";
import { COMPONENT_TYPES } from "@/lib/api/types";
import { BRIEF_MAX, DEFAULT_QUALITY, QUALITY_LEVELS, type QualityLevel } from "@/lib/form-schema";
import { identifierErrorKey, slugifyHint } from "@/lib/validate";
import { useConfirm } from "@/hooks/use-confirm";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

/** Segmented 1-4 candidate count (PRD: 1-4 draft candidates per generation). */
export function CountPicker({
  value,
  onChange,
  testIdPrefix,
}: {
  value: number;
  onChange: (next: number) => void;
  testIdPrefix: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1.5">
      <Label>{t("form.count")}</Label>
      <div
        role="group"
        aria-label={t("form.count")}
        className="grid grid-cols-4 gap-1 rounded-md border p-1"
      >
        {[1, 2, 3, 4].map((count) => (
          <button
            key={count}
            type="button"
            data-testid={`${testIdPrefix}-count-${count}`}
            aria-pressed={value === count}
            onClick={() => onChange(count)}
            className={cn(
              "rounded-sm px-2 py-1 font-mono text-xs transition-colors duration-150 ease-out",
              value === count
                ? "bg-primary font-medium text-primary-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
            )}
          >
            {count}
          </button>
        ))}
      </div>
    </div>
  );
}

/**
 * Segmented quality enum (UI-REVIEW improvement #4: one schema across forms;
 * defaults to the exploration tier `low`, `high` is an explicit choice).
 */
export function QualityPicker({
  value,
  onChange,
  testIdPrefix,
}: {
  value: QualityLevel;
  onChange: (next: QualityLevel) => void;
  testIdPrefix: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1.5">
      <Label>{t("form.quality.label")}</Label>
      <div
        role="group"
        aria-label={t("form.quality.label")}
        className="grid grid-cols-3 gap-1 rounded-md border p-1"
      >
        {QUALITY_LEVELS.map((level) => (
          <button
            key={level}
            type="button"
            data-testid={`${testIdPrefix}-quality-${level}`}
            aria-pressed={value === level}
            onClick={() => onChange(level)}
            className={cn(
              "rounded-sm px-2 py-1 text-xs transition-colors duration-150 ease-out",
              value === level
                ? "bg-primary font-medium text-primary-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
            )}
          >
            {t(`form.quality.${level}`)}
          </button>
        ))}
      </div>
    </div>
  );
}

/**
 * Unified brief textarea (UI-REVIEW improvement #4): shared `BRIEF_MAX` cap
 * live counter, identical across board/page/component forms.
 */
export function BriefField({
  id,
  testId,
  label,
  placeholder,
  value,
  onChange,
  rows = 3,
}: {
  id: string;
  testId: string;
  label: string;
  placeholder: string;
  value: string;
  onChange: (next: string) => void;
  rows?: number;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-baseline justify-between gap-2">
        <Label htmlFor={id}>{label}</Label>
        <span
          data-testid={`${testId}-count`}
          className={cn(
            "font-mono text-[11px]",
            value.length > BRIEF_MAX ? "text-destructive" : "text-muted-foreground",
          )}
        >
          {t("form.briefCount", { count: value.length, max: BRIEF_MAX })}
        </span>
      </div>
      <Textarea
        id={id}
        data-testid={testId}
        value={value}
        maxLength={BRIEF_MAX}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        rows={rows}
      />
    </div>
  );
}

const SLUG_ERROR_KEYS = {
  required: "form.error.slugRequired",
  taken: "form.error.slugTaken",
  format: "form.error.slugFormat",
} as const;

const NAME_ERROR_KEYS = {
  required: "form.error.nameRequired",
  taken: "form.error.nameTaken",
  format: "form.error.nameFormat",
} as const;

/** Add page form (step 3 entry). */
export function AddPageForm({ onAdded }: { onAdded?: () => void }) {
  const { t } = useTranslation();
  const { state, addPage } = useStudio();
  const project = state.project;
  const [slug, setSlug] = useState("");
  const [brief, setBrief] = useState("");
  const [error, setError] = useState<string | null>(null);

  const submit = () => {
    if (!project) return;
    const taken = project.pages.some((page) => page.slug === slug.trim());
    const key = identifierErrorKey(slug, taken, SLUG_ERROR_KEYS);
    if (key) {
      setError(t(key));
      return;
    }
    setError(null);
    void addPage(slug.trim(), brief);
    setSlug("");
    setBrief("");
    onAdded?.();
  };

  return (
    <form
      data-testid="add-page-form"
      className="flex flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="page-slug">{t("form.page.slug")}</Label>
        <Input
          id="page-slug"
          data-testid="page-slug-input"
          value={slug}
          onChange={(event) => {
            setSlug(event.target.value);
            setError(null);
          }}
          onBlur={() => setSlug((prev) => slugifyHint(prev))}
          placeholder={t("form.page.slugPlaceholder")}
          className="font-mono text-xs"
          autoFocus
        />
        <p className="text-[11px] text-muted-foreground">{t("form.page.slugHint")}</p>
        {error && (
          <p data-testid="slug-error" className="text-xs text-destructive">
            {error}
          </p>
        )}
      </div>
      <BriefField
        id="page-brief"
        testId="page-brief-input"
        label={t("form.page.brief")}
        placeholder={t("form.page.briefPlaceholder")}
        value={brief}
        onChange={setBrief}
      />
      <Button type="submit" size="sm" data-testid="add-page-submit">
        {t("form.page.add")}
      </Button>
    </form>
  );
}

/** Selected-page operations: brief, count, quality, regenerate, delete. */
export function PageDetailForm() {
  const { t } = useTranslation();
  const { state, generatePage, deleteArtifact } = useStudio();
  const { ask, element: confirmElement } = useConfirm();
  const project = state.project;
  const page = project?.pages.find((item) => item.slug === state.selectedPage);
  const [brief, setBrief] = useState(page?.brief ?? "");
  const [count, setCount] = useState(2);
  const [quality, setQuality] = useState<QualityLevel>(DEFAULT_QUALITY);

  useEffect(() => {
    setBrief(page?.brief ?? "");
  }, [page?.slug, page?.brief]);

  if (!project || !page) return null;
  const jobRunning = state.job !== null;

  return (
    <div data-testid="page-detail-form" className="flex flex-col gap-3">
      {confirmElement}
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate font-mono text-xs font-semibold">
          {page.slug}
        </span>
        <Button
          variant="ghost"
          size="sm"
          data-testid="page-delete"
          className="text-destructive hover:text-destructive"
          onClick={() =>
            ask({
              title: t("confirm.deleteItem.title"),
              description: t("confirm.deleteItem.desc", { name: page.slug }),
              onConfirm: () => void deleteArtifact({ kind: "page", slug: page.slug }),
            })
          }
        >
          {t("common.delete")}
        </Button>
      </div>
      <BriefField
        id="page-detail-brief"
        testId="page-detail-brief"
        label={t("form.page.brief")}
        placeholder={t("form.page.briefPlaceholder")}
        value={brief}
        onChange={setBrief}
      />
      <CountPicker value={count} onChange={setCount} testIdPrefix="page-detail" />
      <QualityPicker value={quality} onChange={setQuality} testIdPrefix="page-detail" />
      <Button
        size="sm"
        data-testid="page-generate"
        disabled={jobRunning || !brief.trim()}
        onClick={() => void generatePage(page.slug, count, quality)}
      >
        {page.candidates.length > 0 || page.current
          ? t("form.page.regenerate")
          : t("form.page.generate")}
      </Button>
      <p className="text-[11px] text-muted-foreground">{t("form.page.hint")}</p>
    </div>
  );
}

/** Add component form (step 4 entry) with the seven type choices. */
export function AddComponentForm({ onAdded }: { onAdded?: () => void }) {
  const { t } = useTranslation();
  const { state, addComponent } = useStudio();
  const project = state.project;
  const [name, setName] = useState("");
  const [type, setType] = useState<ComponentType>("buttons");
  const [brief, setBrief] = useState("");
  const [nameError, setNameError] = useState<string | null>(null);
  const [briefError, setBriefError] = useState<string | null>(null);

  const submit = () => {
    if (!project) return;
    const trimmed = name.trim();
    const taken = project.components.some((component) => component.name === trimmed);
    const nameKey = identifierErrorKey(trimmed, taken, NAME_ERROR_KEYS);
    if (nameKey) {
      setNameError(t(nameKey));
      return;
    }
    if (!brief.trim()) {
      setBriefError(t("form.error.briefRequired"));
      return;
    }
    setNameError(null);
    setBriefError(null);
    void addComponent(trimmed, type, brief);
    setName("");
    setBrief("");
    onAdded?.();
  };

  return (
    <form
      data-testid="add-component-form"
      className="flex flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="component-name">{t("form.component.name")}</Label>
        <Input
          id="component-name"
          data-testid="component-name-input"
          value={name}
          onChange={(event) => {
            setName(event.target.value);
            setNameError(null);
          }}
          onBlur={() => setName((prev) => slugifyHint(prev))}
          placeholder={t("form.component.namePlaceholder")}
          className="font-mono text-xs"
          autoFocus
        />
        <p className="text-[11px] text-muted-foreground">{t("form.component.nameHint")}</p>
        {nameError && (
          <p data-testid="name-error" className="text-xs text-destructive">
            {nameError}
          </p>
        )}
      </div>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="component-type">{t("form.component.type")}</Label>
        <Select
          id="component-type"
          data-testid="component-type-select"
          value={type}
          onChange={(event) => setType(event.target.value as ComponentType)}
        >
          {COMPONENT_TYPES.map((option) => (
            <option key={option} value={option}>
              {t(`component.type.${option}`)}
            </option>
          ))}
        </Select>
      </div>
      <div className="flex flex-col gap-1.5">
        <BriefField
          id="component-brief"
          testId="component-brief-input"
          label={t("form.component.brief")}
          placeholder={t("form.component.briefPlaceholder")}
          value={brief}
          onChange={(next) => {
            setBrief(next);
            setBriefError(null);
          }}
        />
        {briefError && (
          <p data-testid="brief-error" className="text-xs text-destructive">
            {briefError}
          </p>
        )}
      </div>
      <Button type="submit" size="sm" data-testid="add-component-submit">
        {t("form.component.add")}
      </Button>
    </form>
  );
}

/** Selected-component operations: type, brief, count, quality, regenerate. */
export function ComponentDetailForm() {
  const { t } = useTranslation();
  const { state, generateComponent, deleteArtifact } = useStudio();
  const { ask, element: confirmElement } = useConfirm();
  const project = state.project;
  const component = project?.components.find(
    (item) => item.name === state.selectedComponent,
  );
  const [brief, setBrief] = useState(component?.brief ?? "");
  const [count, setCount] = useState(2);
  const [quality, setQuality] = useState<QualityLevel>(DEFAULT_QUALITY);

  useEffect(() => {
    setBrief(component?.brief ?? "");
  }, [component?.name, component?.brief]);

  if (!project || !component) return null;
  const jobRunning = state.job !== null;

  return (
    <div data-testid="component-detail-form" className="flex flex-col gap-3">
      {confirmElement}
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate font-mono text-xs font-semibold">
          {component.name}
        </span>
        <span className="shrink-0 rounded-full border px-2 py-0.5 text-[11px] text-muted-foreground">
          {t(`component.type.${component.type}`)}
        </span>
        <Button
          variant="ghost"
          size="sm"
          data-testid="component-delete"
          className="text-destructive hover:text-destructive"
          onClick={() =>
            ask({
              title: t("confirm.deleteItem.title"),
              description: t("confirm.deleteItem.desc", { name: component.name }),
              onConfirm: () => void deleteArtifact({ kind: "component", name: component.name }),
            })
          }
        >
          {t("common.delete")}
        </Button>
      </div>
      <BriefField
        id="component-detail-brief"
        testId="component-detail-brief"
        label={t("form.component.brief")}
        placeholder={t("form.component.briefPlaceholder")}
        value={brief}
        onChange={setBrief}
      />
      <CountPicker value={count} onChange={setCount} testIdPrefix="component-detail" />
      <QualityPicker value={quality} onChange={setQuality} testIdPrefix="component-detail" />
      <Button
        size="sm"
        data-testid="component-generate"
        disabled={jobRunning || !brief.trim()}
        onClick={() => void generateComponent(component.name, count, quality)}
      >
        {component.candidates.length > 0 || component.current
          ? t("form.component.regenerate")
          : t("form.component.generate")}
      </Button>
      <p className="text-[11px] text-muted-foreground">{t("form.component.hint")}</p>
    </div>
  );
}
