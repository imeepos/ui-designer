import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderOutput, Settings } from "lucide-react";
import pkg from "../package.json";

import { HelmMark } from "@/components/icons";
import { LanguageSwitcher } from "@/components/language-switcher";
import { CreateProjectDialog } from "@/components/create-project-dialog";
import { ExportDialog } from "@/components/export-dialog";
import { SettingsDialog } from "@/components/settings-dialog";
import { DetailPanel } from "@/components/panels/detail-panel";
import { GalleryPanel } from "@/components/panels/gallery-panel";
import { ProjectsPanel } from "@/components/panels/projects-panel";
import {
  Stepper,
  computeStepStates,
  stepUnlockedMap,
} from "@/components/stepper";
import { ThemeToggle } from "@/components/theme-toggle";
import { Button } from "@/components/ui/button";
import { useCoreStatus } from "@/hooks/use-core-status";
import { OPEN_SETTINGS_EVENT } from "@/lib/events";
import { StudioProvider, useStudio } from "@/state/studio";
import { ToastProvider } from "@/state/toast";

export default function App() {
  return (
    <ToastProvider>
      <StudioProvider>
        <AppShell />
      </StudioProvider>
    </ToastProvider>
  );
}

function AppShell() {
  const { t } = useTranslation();
  const core = useCoreStatus();
  const { state, apiMode, setView } = useStudio();
  const [createOpen, setCreateOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Global toast actions (NO_CREDENTIALS) ask for Settings via CustomEvent.
  useEffect(() => {
    const openSettings = () => setSettingsOpen(true);
    window.addEventListener(OPEN_SETTINGS_EVENT, openSettings);
    return () => window.removeEventListener(OPEN_SETTINGS_EVENT, openSettings);
  }, []);

  const stepStates = computeStepStates(state.project, state.view);
  const unlocked = stepUnlockedMap(state.project);

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
          <Button
            variant="outline"
            size="sm"
            data-testid="export-open"
            disabled={state.project === null}
            onClick={() => setExportOpen(true)}
          >
            <FolderOutput className="size-3.5" />
            {t("common.export")}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            data-testid="settings-open"
            aria-label={t("settings.open")}
            title={t("settings.open")}
            onClick={() => setSettingsOpen(true)}
          >
            <Settings className="size-4" />
          </Button>
          <LanguageSwitcher />
          <ThemeToggle />
        </div>
      </header>

      <Stepper
        current={state.view}
        states={stepStates}
        unlocked={unlocked}
        onSelect={setView}
      />

      <main className="flex min-h-0 flex-1">
        <ProjectsPanel />
        <GalleryPanel onNewProject={() => setCreateOpen(true)} />
        <DetailPanel onNewProject={() => setCreateOpen(true)} />
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
        <span data-testid="api-mode" className="font-mono">
          {apiMode === "mock" ? t("status.api.mock") : t("status.api.tauri")}
        </span>
        <span className="ml-auto font-mono">v{pkg.version}</span>
      </footer>

      <CreateProjectDialog open={createOpen} onClose={() => setCreateOpen(false)} />
      <ExportDialog open={exportOpen} onClose={() => setExportOpen(false)} />
      <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}
