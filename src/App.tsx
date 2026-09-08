import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import { HelmMark } from "@/components/icons";
import { LanguageSwitcher } from "@/components/language-switcher";
import { DetailPanel } from "@/components/panels/detail-panel";
import { GalleryPanel } from "@/components/panels/gallery-panel";
import { ProjectsPanel } from "@/components/panels/projects-panel";
import { Stepper, type StepId } from "@/components/stepper";
import { ThemeToggle } from "@/components/theme-toggle";
import { useCoreStatus } from "@/hooks/use-core-status";

export default function App() {
  const { t } = useTranslation();
  const core = useCoreStatus();
  // Phase 1: stays on the first step statically; Phase 3 wires real project state
  const currentStep: StepId = "project";

  useEffect(() => {
    document.title = t("app.title");
  }, [t]);

  return (
    <div
      data-testid="app-shell"
      className="flex h-screen min-h-0 flex-col bg-background text-foreground"
    >
      <header className="flex h-12 shrink-0 items-center justify-between gap-4 border-b px-4">
        <div className="flex min-w-0 items-center gap-2">
          <HelmMark className="size-5 shrink-0 text-primary" />
          <span className="truncate text-sm font-semibold">{t("app.title")}</span>
          <span className="hidden truncate text-xs text-muted-foreground md:inline">
            {t("app.subtitle")}
          </span>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <LanguageSwitcher />
          <ThemeToggle />
        </div>
      </header>

      <Stepper current={currentStep} />

      <main className="flex min-h-0 flex-1">
        <ProjectsPanel />
        <GalleryPanel />
        <DetailPanel />
      </main>

      <footer className="flex h-8 shrink-0 items-center gap-2 border-t px-4 text-xs text-muted-foreground">
        <span>{t("status.core.label")}</span>
        <span data-testid="core-status" className="font-mono">
          {core.state === "ok"
            ? t("status.core.ok", { version: core.version })
            : core.state === "browser"
              ? t("status.core.browser")
              : t("status.core.checking")}
        </span>
        <span className="ml-auto font-mono">{t("footer.phase")}</span>
      </footer>
    </div>
  );
}
