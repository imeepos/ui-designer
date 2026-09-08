import { useEffect, useMemo, useState } from "react";
import { FileJson, FileText, FolderOpen, Image } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import type { ExportedFile, ExportResult } from "@/lib/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Modal } from "@/components/ui/modal";
import { JobPanel } from "@/components/job-panel";

type ExportDialogProps = {
  open: boolean;
  onClose: () => void;
};

/** Export: mock directory pick + progress + artifact manifest list. */
export function ExportDialog({ open, onClose }: ExportDialogProps) {
  const { t } = useTranslation();
  const { state, exportProject } = useStudio();
  const project = state.project;

  const [outDir, setOutDir] = useState("");
  const [phase, setPhase] = useState<"form" | "done">("form");
  // Snapshot of the export this dialog instance produced (guards against
  // showing a previous project's result).
  const [result, setResult] = useState<ExportResult | null>(null);

  useEffect(() => {
    if (!open) return;
    setPhase("form");
    setResult(null);
    setOutDir(project ? `~/Rudder/exports/${project.name}` : "~/Rudder/exports");
  }, [open, project]);

  useEffect(() => {
    if (open && state.lastExport && !state.job) {
      setResult(state.lastExport);
      setPhase("done");
    }
  }, [open, state.lastExport, state.job]);

  const files = result?.files ?? [];
  const totals = useMemo(() => {
    const images = files.filter((file) => file.kind === "image");
    return {
      images: images.length,
      bytes: files.reduce((sum, file) => sum + file.bytes, 0),
    };
  }, [files]);

  const start = () => {
    void exportProject(outDir);
  };

  if (!project) return null;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("export.dialog.title")}
      testId="export-dialog"
      wide={phase === "done"}
      footer={
        phase === "form" ? (
          <>
            <Button variant="outline" size="sm" onClick={onClose}>
              {t("common.cancel")}
            </Button>
            <Button size="sm" data-testid="export-start" onClick={start} disabled={state.job !== null}>
              {t("export.dialog.start")}
            </Button>
          </>
        ) : (
          <Button size="sm" data-testid="export-done" onClick={onClose}>
            {t("common.done")}
          </Button>
        )
      }
    >
      {phase === "form" ? (
        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="export-dir">{t("export.dialog.dir")}</Label>
            <div className="flex items-center gap-2">
              <Input
                id="export-dir"
                data-testid="export-dir-input"
                value={outDir}
                onChange={(event) => setOutDir(event.target.value)}
                className="font-mono text-xs"
              />
              <Button
                variant="outline"
                size="sm"
                data-testid="export-browse"
                onClick={() => setOutDir((prev) => prev)}
                title={t("export.dialog.browseHint")}
              >
                <FolderOpen className="size-3.5" />
                {t("export.dialog.browse")}
              </Button>
            </div>
            <p className="text-[10px] text-muted-foreground">{t("export.dialog.dirHint")}</p>
          </div>
          {state.job?.kind === "export" ? (
            <JobPanel job={state.job} />
          ) : (
            <p className="text-xs text-muted-foreground">{t("export.dialog.summary", {
              pages: project.pages.length,
              components: project.components.length,
            })}</p>
          )}
        </div>
      ) : (
        <div className="flex flex-col gap-3" data-testid="export-result">
          <div className="flex items-center gap-2 rounded-md border bg-muted/40 px-3 py-2 text-xs">
            <Image aria-hidden="true" className="size-4 text-primary" />
            <span>{t("export.dialog.doneSummary", { images: totals.images, files: files.length })}</span>
            <span className="ml-auto font-mono text-[10px] text-muted-foreground">
              {formatBytes(totals.bytes)}
            </span>
          </div>
          <p className="font-mono text-[10px] text-muted-foreground">{result?.outDir}</p>
          <ul className="flex flex-col gap-1" data-testid="export-files">
            {files.map((file) => (
              <li
                key={file.path}
                className="flex items-center gap-2 rounded-sm border px-2.5 py-1.5 text-xs"
              >
                <FileGlyph kind={file.kind} />
                <span className="min-w-0 flex-1 truncate font-mono text-[11px]">{file.path}</span>
                <span className="shrink-0 font-mono text-[10px] text-muted-foreground">
                  {formatBytes(file.bytes)}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </Modal>
  );
}

function FileGlyph({ kind }: { kind: ExportedFile["kind"] }) {
  if (kind === "image") return <Image aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />;
  if (kind === "prompts") return <FileText aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />;
  return <FileJson aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1000))} KB`;
}
