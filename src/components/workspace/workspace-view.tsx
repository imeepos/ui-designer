import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { AddComponentForm, AddPageForm } from "@/components/detail-forms";
import { Modal } from "@/components/ui/modal";
import { LeftMenu } from "@/components/workspace/left-menu";
import { Stage } from "@/components/workspace/stage";
import type { LineageTarget } from "@/lib/api/types";
import { LineagePanel } from "@/components/workspace/lineage-panel";
import type { RegenMode } from "@/components/workspace/regen-drawer";
import { RegenDrawer } from "@/components/workspace/regen-drawer";
import type { SwitchMode } from "@/components/workspace/switch-dialog";
import { SwitchDialog } from "@/components/workspace/switch-dialog";

/**
 * Project workspace: left menu (search + overview + groups) and the center
 * stage; regenerate drawer / switch dialog / lineage panel / add dialogs
 * live here.
 */
export function WorkspaceView() {
  const { t } = useTranslation();
  const { state } = useStudio();
  const [regenMode, setRegenMode] = useState<RegenMode | null>(null);
  const [switchMode, setSwitchMode] = useState<SwitchMode | null>(null);
  const [addKind, setAddKind] = useState<"page" | "component" | null>(null);
  const [lineageTarget, setLineageTarget] = useState<LineageTarget | null>(null);

  const view = state.view;
  const regenForView: RegenMode =
    view === "overview" ? "board" : view.startsWith("page:") ? { kind: "page", slug: view.slice(5) } : { kind: "component", name: view.slice(10) };
  const switchForView: SwitchMode =
    view === "overview" ? "board" : view.startsWith("page:") ? { kind: "page", slug: view.slice(5) } : { kind: "component", name: view.slice(10) };

  return (
    <main className="flex min-h-0 flex-1" data-testid="workspace-view">
      <LeftMenu
        onAddPage={() => setAddKind("page")}
        onAddComponent={() => setAddKind("component")}
      />
      <Stage
        onRegenerate={() => setRegenMode(regenForView)}
        onSwitch={() => setSwitchMode(switchForView)}
        onLineage={setLineageTarget}
      />

      {regenMode && <RegenDrawer mode={regenMode} onClose={() => setRegenMode(null)} />}
      {switchMode && (
        <SwitchDialog mode={switchMode} onClose={() => setSwitchMode(null)} />
      )}
      {lineageTarget && (
        <LineagePanel target={lineageTarget} onClose={() => setLineageTarget(null)} />
      )}

      <Modal
        open={addKind === "page"}
        onClose={() => setAddKind(null)}
        title={t("workspace.addPage")}
        testId="add-page-dialog"
      >
        {addKind === "page" && <AddPageForm onAdded={() => setAddKind(null)} />}
      </Modal>
      <Modal
        open={addKind === "component"}
        onClose={() => setAddKind(null)}
        title={t("workspace.addComponent")}
        testId="add-component-dialog"
      >
        {addKind === "component" && <AddComponentForm onAdded={() => setAddKind(null)} />}
      </Modal>
    </main>
  );
}
