import { useTranslation } from "react-i18next";

import type { ComponentItem } from "@/lib/api/types";
import { useStudio } from "@/state/studio";
import { ArtifactFlow, type FlowItem } from "@/components/gallery/artifact-flow";

function toFlowItem(component: ComponentItem): FlowItem {
  return {
    id: component.name,
    label: component.name,
    current: component.current,
    candidates: component.candidates,
    history: component.history,
  };
}

/** Component view (step 4): same lifecycle, with a type badge. */
export function ComponentsSection() {
  const { t } = useTranslation();
  const { state, selectComponent, pickComponent, deleteArtifact } = useStudio();
  const project = state.project;
  if (!project) return null;

  const items = project.components.map((component) => ({
    ...toFlowItem(component),
    meta: (
      <span className="shrink-0 rounded-full border px-1.5 text-[9px] tracking-wide text-muted-foreground">
        {t(`component.type.${component.type}`)}
      </span>
    ),
  }));

  return (
    <ArtifactFlow
      testPrefix="component"
      items={items}
      selectedId={state.selectedComponent}
      onSelect={(name) => selectComponent(name)}
      onPick={(name, candidateId) => void pickComponent(name, candidateId)}
      onDeleteItem={(name) => void deleteArtifact({ kind: "component", name })}
      onDeleteHistory={(name, ts) => void deleteArtifact({ kind: "componentHistory", name, ts })}
      jobTarget={state.job?.kind === "component" ? state.job.target : undefined}
      emptyTitle={t("gallery.components.empty.title")}
      emptyDesc={t("gallery.components.empty.desc")}
    />
  );
}
