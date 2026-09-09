import { useEffect, useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import type { BoardBrief } from "@/lib/api/types";
import { BRIEF_MAX, DEFAULT_QUALITY, type QualityLevel } from "@/lib/form-schema";
import { formatSize } from "@/lib/size";
import { OPEN_TREE_ADD_EVENT } from "@/lib/events";
import {
  AddComponentForm,
  AddPageForm,
  BriefField,
  ComponentDetailForm,
  CountPicker,
  PageDetailForm,
  QualityPicker,
} from "@/components/detail-forms";
import { JobPanel } from "@/components/job-panel";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

/** Right column: forms & actions for the selected tree node (320px, THEME §4). */
export function DetailPanel({ onNewProject }: { onNewProject: () => void }) {
  const { t } = useTranslation();
  const { state } = useStudio();
  const project = state.project;
  const view = state.view;

  let content;
  if (!project) {
    content = (
      <Card data-testid="detail-placeholder">
        <CardHeader>
          <CardTitle>{t("panel.detail.placeholder.title")}</CardTitle>
          <CardDescription>{t("panel.detail.empty")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Button size="sm" onClick={onNewProject}>
            {t("panel.gallery.empty.cta")}
          </Button>
        </CardContent>
      </Card>
    );
  } else if (view.startsWith("component:")) {
    content = <ComponentWorkbench />;
  } else if (view.startsWith("page:")) {
    content = <PageWorkbench />;
  } else {
    switch (view) {
      case "overview":
        content = (
          <>
            <ProjectMetaCard />
            <BoardFormCard />
          </>
        );
        break;
      case "pages":
        content = <PageWorkbench />;
        break;
      case "components":
        content = <ComponentWorkbench />;
        break;
    }
  }

  return (
    <aside
      data-testid="detail-panel"
      className="flex w-80 shrink-0 flex-col border-l"
    >
      <div className="flex h-10 shrink-0 items-center px-4">
        <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("panel.detail.title")}
        </h2>
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4 pt-0">
        {content}
      </div>
    </aside>
  );
}

function ProjectMetaCard() {
  const { t } = useTranslation();
  const { state } = useStudio();
  const project = state.project;
  if (!project) return null;

  return (
    <Card data-testid="project-meta">
      <CardHeader>
        <CardTitle>{project.name}</CardTitle>
        <CardDescription>{t("panel.detail.project.created", {
          date: new Date(project.createdAt).toLocaleDateString(),
        })}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-2 text-xs">
        <Row label={t("project.form.size")} value={formatSize(project.size)} />
        <Row label={t("panel.detail.project.brand")} value={project.brandBrief || "—"} />
        <Row
          label={t("panel.detail.project.style")}
          value={project.styleBrief || "—"}
        />
        <Row
          label={t("panel.detail.project.pages")}
          value={String(project.pages.length)}
        />
        <Row
          label={t("panel.detail.project.components")}
          value={String(project.components.length)}
        />
      </CardContent>
    </Card>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <span className="shrink-0 text-muted-foreground">{label}</span>
      <span className="min-w-0 break-words text-right font-mono text-[11px]">{value}</span>
    </div>
  );
}

/** Step 2 right panel: board brief form + generate (with progress + cancel). */
function BoardFormCard() {
  const { t } = useTranslation();
  const { state, generateBoard } = useStudio();
  const project = state.project;
  const [brandKeywords, setBrandKeywords] = useState(project?.brandBrief ?? "");
  const [colorDirection, setColorDirection] = useState("");
  const [fontMood, setFontMood] = useState("");
  const [radiusDensity, setRadiusDensity] = useState("");
  const [reference, setReference] = useState("");
  const [count, setCount] = useState(2);
  const [quality, setQuality] = useState<QualityLevel>(DEFAULT_QUALITY);

  if (!project) return null;
  const boardJob = state.job?.kind === "board" ? state.job : null;

  const submit = () => {
    const brief: BoardBrief = {
      brandKeywords,
      colorDirection,
      fontMood,
      radiusDensity,
      reference,
    };
    void generateBoard(brief, count, quality);
  };

  return (
    <Card data-testid="board-form">
      <CardHeader>
        <CardTitle>{t("form.board.title")}</CardTitle>
        <CardDescription>{t("form.board.desc")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <BriefField
          id="board-brand"
          testId="board-brand-input"
          label={t("form.board.brandKeywords")}
          placeholder={t("form.board.brandPlaceholder")}
          value={brandKeywords}
          onChange={setBrandKeywords}
          rows={2}
        />
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="board-color">{t("form.board.colorDirection")}</Label>
          <Input
            id="board-color"
            data-testid="board-color-input"
            value={colorDirection}
            maxLength={BRIEF_MAX}
            onChange={(event) => setColorDirection(event.target.value)}
            placeholder={t("form.board.colorPlaceholder")}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="board-font">{t("form.board.fontMood")}</Label>
          <Input
            id="board-font"
            data-testid="board-font-input"
            value={fontMood}
            maxLength={BRIEF_MAX}
            onChange={(event) => setFontMood(event.target.value)}
            placeholder={t("form.board.fontPlaceholder")}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="board-radius">{t("form.board.radiusDensity")}</Label>
          <Input
            id="board-radius"
            data-testid="board-radius-input"
            value={radiusDensity}
            maxLength={BRIEF_MAX}
            onChange={(event) => setRadiusDensity(event.target.value)}
            placeholder={t("form.board.radiusPlaceholder")}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="board-reference">{t("form.board.reference")}</Label>
          <Textarea
            id="board-reference"
            data-testid="board-reference-input"
            value={reference}
            maxLength={BRIEF_MAX}
            onChange={(event) => setReference(event.target.value)}
            placeholder={t("form.board.referencePlaceholder")}
            rows={2}
          />
        </div>
        <CountPicker value={count} onChange={setCount} testIdPrefix="board" />
        <QualityPicker value={quality} onChange={setQuality} testIdPrefix="board" />
        {boardJob ? (
          <JobPanel job={boardJob} />
        ) : (
          <Button
            size="sm"
            data-testid="board-generate"
            disabled={!brandKeywords.trim() || state.job !== null}
            onClick={submit}
          >
            {project.boardCandidates.length > 0
              ? t("form.board.regenerate")
              : t("form.board.generate")}
          </Button>
        )}
        {project.boardCandidates.length > 0 && !project.anchor && (
          <p className="text-[11px] text-muted-foreground">{t("form.board.anchorHint")}</p>
        )}
      </CardContent>
    </Card>
  );
}

function PageWorkbench() {
  const { t } = useTranslation();
  const { state, selectPage } = useStudio();
  const project = state.project;
  const [adding, setAdding] = useState(false);

  // Tree "+" entry asks this workbench to reveal the add-page form.
  useEffect(() => {
    const open = (event: Event) => {
      if ((event as CustomEvent<{ kind?: string }>).detail?.kind === "page") {
        setAdding(true);
      }
    };
    window.addEventListener(OPEN_TREE_ADD_EVENT, open);
    return () => window.removeEventListener(OPEN_TREE_ADD_EVENT, open);
  }, []);

  if (!project) return null;

  const selectedPage = project.pages.find((page) => page.slug === state.selectedPage);

  return (
    <>
      {selectedPage ? (
        <Card data-testid="page-detail-card">
          <CardHeader>
            <CardTitle>{t("form.page.detailTitle")}</CardTitle>
          </CardHeader>
          <CardContent>
            <PageDetailForm />
          </CardContent>
        </Card>
      ) : (
        project.pages.length > 0 && (
          <Card data-testid="page-quick-list">
            <CardHeader>
              <CardTitle>{t("form.page.pickTitle")}</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-wrap gap-1.5">
              {project.pages.map((page) => (
                <Button
                  key={page.slug}
                  variant="outline"
                  size="sm"
                  className="font-mono text-[11px]"
                  onClick={() => selectPage(page.slug)}
                >
                  {page.slug}
                </Button>
              ))}
            </CardContent>
          </Card>
        )
      )}

      <Card>
        <CardHeader>
          <button
            type="button"
            data-testid="add-page-toggle"
            onClick={() => setAdding((prev) => !prev)}
            className="flex items-center gap-1.5 text-sm font-semibold text-foreground"
          >
            {adding ? (
              <ChevronDown className="size-3.5 text-muted-foreground" />
            ) : (
              <ChevronRight className="size-3.5 text-muted-foreground" />
            )}
            {t("form.page.addTitle")}
          </button>
        </CardHeader>
        {adding && (
          <CardContent className={cn("flex flex-col gap-3")}>
            <AddPageForm onAdded={() => setAdding(false)} />
          </CardContent>
        )}
      </Card>

      {state.job?.kind === "page" && <JobPanel job={state.job} />}
    </>
  );
}

function ComponentWorkbench() {
  const { t } = useTranslation();
  const { state, selectComponent } = useStudio();
  const project = state.project;
  const [adding, setAdding] = useState(false);

  // Tree "+" entry asks this workbench to reveal the add-component form.
  useEffect(() => {
    const open = (event: Event) => {
      if ((event as CustomEvent<{ kind?: string }>).detail?.kind === "component") {
        setAdding(true);
      }
    };
    window.addEventListener(OPEN_TREE_ADD_EVENT, open);
    return () => window.removeEventListener(OPEN_TREE_ADD_EVENT, open);
  }, []);

  if (!project) return null;

  const selectedComponent = project.components.find(
    (component) => component.name === state.selectedComponent,
  );

  return (
    <>
      {selectedComponent ? (
        <Card data-testid="component-detail-card">
          <CardHeader>
            <CardTitle>{t("form.component.detailTitle")}</CardTitle>
          </CardHeader>
          <CardContent>
            <ComponentDetailForm />
          </CardContent>
        </Card>
      ) : (
        project.components.length > 0 && (
          <Card data-testid="component-quick-list">
            <CardHeader>
              <CardTitle>{t("form.component.pickTitle")}</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-wrap gap-1.5">
              {project.components.map((component) => (
                <Button
                  key={component.name}
                  variant="outline"
                  size="sm"
                  className="font-mono text-[11px]"
                  onClick={() => selectComponent(component.name)}
                >
                  {component.name}
                </Button>
              ))}
            </CardContent>
          </Card>
        )
      )}

      <Card>
        <CardHeader>
          <button
            type="button"
            data-testid="add-component-toggle"
            onClick={() => setAdding((prev) => !prev)}
            className="flex items-center gap-1.5 text-sm font-semibold text-foreground"
          >
            {adding ? (
              <ChevronDown className="size-3.5 text-muted-foreground" />
            ) : (
              <ChevronRight className="size-3.5 text-muted-foreground" />
            )}
            {t("form.component.addTitle")}
          </button>
        </CardHeader>
        {adding && (
          <CardContent className={cn("flex flex-col gap-3")}>
            <AddComponentForm onAdded={() => setAdding(false)} />
          </CardContent>
        )}
      </Card>

      {state.job?.kind === "component" && <JobPanel job={state.job} />}
    </>
  );
}
