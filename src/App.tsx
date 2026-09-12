import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, FolderOutput, Settings } from "lucide-react";
import pkg from "../package.json";

import { HelmMark } from "@/components/icons";
import { LanguageSwitcher } from "@/components/language-switcher";
import { AnchorBadge } from "@/components/anchor-badge";
import { CreateWizard } from "@/components/create-wizard";
import { HomeView } from "@/components/home-view";
import { WorkspaceView } from "@/components/workspace/workspace-view";
import { ExportDialog } from "@/components/export-dialog";
import { SettingsDialog } from "@/components/settings-dialog";
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
  const { state, apiMode, closeProject } = useStudio();
  const [wizardOpen, setWizardOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Global toast actions (NO_CREDENTIALS) ask for Settings via CustomEvent.
  useEffect(() => {
    const openSettings = () => setSettingsOpen(true);
    window.addEventListener(OPEN_SETTINGS_EVENT, openSettings);
    return () => window.removeEventListener(OPEN_SETTINGS_EVENT, openSettings);
  }, []);

  useEffect(() => {
    document.title = t("app.title");
  }, [t]);

  const inWorkspace = state.project !== null && state.entered;

  return (
    <div
      data-testid="app-shell"
      className="flex h-screen min-h-0 flex-col bg-background text-foreground"
    >
      <header className="flex h-12 shrink-0 items-center justify-between gap-4 border-b px-4">
        <div className="flex min-w-0 items-center gap-2">
          {inWorkspace && (
            <Button
              variant="ghost"
              size="icon"
              data-testid="back-home"
              aria-label={t("workspace.backHome")}
              title={t("workspace.backHome")}
              onClick={closeProject}
            >
              <ArrowLeft className="size-4" />
            </Button>
          )}
          <HelmMark className="size-5 shrink-0 text-primary" />
          <span className="truncate font-serif text-sm font-semibold tracking-wide">{t("app.title")}</span>
          {inWorkspace && state.project ? (
            <span className="flex min-w-0 items-center gap-1.5">
              <span aria-hidden="true" className="text-muted-foreground">/</span>
              <span className="min-w-0 truncate text-sm text-muted-foreground">
                {state.project.name}
              </span>
              {state.project.anchor && <AnchorBadge className="scale-90" />}
            </span>
          ) : (
            <span className="hidden truncate text-xs text-muted-foreground md:inline">
              {t("app.subtitle")}
            </span>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {inWorkspace && (
            <Button
              variant="outline"
              size="sm"
              data-testid="export-open"
              onClick={() => setExportOpen(true)}
            >
              <FolderOutput className="size-3.5" />
              {t("common.export")}
            </Button>
          )}
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

      {inWorkspace ? (
        <WorkspaceView />
      ) : (
        <HomeView onCreate={() => setWizardOpen(true)} />
      )}

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

      <CreateWizard open={wizardOpen} onClose={() => setWizardOpen(false)} />
      <ExportDialog open={exportOpen} onClose={() => setExportOpen(false)} />
      <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}
