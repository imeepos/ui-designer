import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ActiveJob, JobKind } from "@/state/studio";
import { useStudio } from "@/state/studio";
import { Button } from "@/components/ui/button";

/** THEME §5: skeleton + one muted line, cancellable; mentions the 3-minute cap. */
export function JobPanel({ job }: { job: ActiveJob }) {
  const { t } = useTranslation();
  const { cancelJob } = useStudio();
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  const elapsedSeconds = Math.max(0, Math.floor((now - job.startedAt) / 1000));
  const percent = Math.round(job.progress * 100);

  return (
    <div
      data-testid="job-panel"
      data-job-kind={job.kind}
      className="flex flex-col gap-2 rounded-lg border bg-muted/40 p-3"
    >
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Loader2 aria-hidden="true" className="size-3.5 animate-spin text-primary" />
        <span className="min-w-0 flex-1 truncate">{jobLabel(t, job.kind, job.target)}</span>
        <span className="font-mono tabular-nums">{t("job.elapsed", { seconds: elapsedSeconds })}</span>
      </div>
      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        className="h-1 w-full overflow-hidden rounded-full bg-muted"
      >
        <div
          className="h-full rounded-full bg-primary transition-[width] duration-200 ease-out"
          style={{ width: `${Math.max(percent, 4)}%` }}
        />
      </div>
      <div className="flex items-center justify-between gap-2">
        <span className="text-[10px] tracking-wide text-muted-foreground">
          {t("job.etaNote")}
        </span>
        <Button variant="ghost" size="sm" data-testid="job-cancel" onClick={cancelJob}>
          {t("common.cancel")}
        </Button>
      </div>
    </div>
  );
}

function jobLabel(
  t: (key: string, options?: Record<string, unknown>) => string,
  kind: JobKind,
  target?: string,
): string {
  switch (kind) {
    case "board":
      return t("job.label.board");
    case "page":
      return t("job.label.page", { slug: target ?? "" });
    case "component":
      return t("job.label.component", { name: target ?? "" });
    case "export":
      return t("job.label.export");
  }
}
