import { useTranslation } from "react-i18next";

import type { PageItem } from "@/lib/api/types";
import { useStudio } from "@/state/studio";
import { ArtifactFlow, type FlowItem } from "@/components/gallery/artifact-flow";

function toFlowItem(page: PageItem): FlowItem {
  return {
    id: page.slug,
    label: page.slug,
    current: page.current,
    candidates: page.candidates,
    history: page.history,
  };
}

/** Page view (step 3): grid -> candidates -> pick -> history -> delete. */
export function PagesSection() {
  const { t } = useTranslation();
  const { state, selectPage, pickPage, deleteArtifact } = useStudio();
  const project = state.project;
  if (!project) return null;

  return (
    <ArtifactFlow
      testPrefix="page"
      items={project.pages.map(toFlowItem)}
      selectedId={state.selectedPage}
      onSelect={(slug) => selectPage(slug)}
      onPick={(slug, candidateId) => void pickPage(slug, candidateId)}
      onDeleteItem={(slug) => void deleteArtifact({ kind: "page", slug })}
      onDeleteHistory={(slug, ts) => void deleteArtifact({ kind: "pageHistory", slug, ts })}
      jobTarget={state.job?.kind === "page" ? state.job.target : undefined}
      emptyTitle={t("gallery.pages.empty.title")}
      emptyDesc={t("gallery.pages.empty.desc")}
    />
  );
}
