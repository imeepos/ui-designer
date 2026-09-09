import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Copy, Waypoints, X as XIcon } from "lucide-react";

import type { GenRecord } from "@/lib/api/types";
import { copyText } from "@/lib/copy";
import { useStudio } from "@/state/studio";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { LineageTarget } from "@/lib/api/types";

/**
 * Stage lineage panel (right slide-in): the GenRecord batch behind the
 * current image — full prompt, params table and source badge. Exactly one
 * entry point (the stage button), one panel, two copy actions
 * (prompt text / whole record JSON).
 */
export function LineagePanel({ target, onClose }: { target: LineageTarget; onClose: () => void }) {
  const { t } = useTranslation();
  const { state, getLineage } = useStudio();
  const projectId = state.project?.id;
  const [record, setRecord] = useState<GenRecord | null>(null);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState<"prompt" | "json" | null>(null);

  useEffect(() => {
    if (!projectId) return;
    let cancelled = false;
    setLoading(true);
    setRecord(null);
    getLineage(target)
      .then((found) => {
        if (!cancelled) setRecord(found);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [projectId, target, getLineage]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const flashCopied = (which: "prompt" | "json") => {
    setCopied(which);
    window.setTimeout(() => {
      setCopied((current) => (current === which ? null : current));
    }, 1500);
  };

  const copyPrompt = async () => {
    if (record && (await copyText(record.prompt))) flashCopied("prompt");
  };

  const copyJson = async () => {
    if (record && (await copyText(JSON.stringify(record, null, 2)))) flashCopied("json");
  };

  const paramRows: Array<[string, string]> = [];
  if (record) {
    const rows: Array<[string, string | undefined]> = [
      ["endpoint", record.endpoint],
      ["template", record.templateId],
      ["model", record.params.model],
      ["size", record.params.size],
      ["quality", record.params.quality],
      ["n", String(record.params.n)],
      ["seed", record.params.seed !== undefined ? String(record.params.seed) : undefined],
      ["thinking", record.params.thinking],
      ["candidates", record.candidateIds.join(", ")],
      ["generatedAt", record.at],
    ];
    for (const row of rows) {
      if (row[1] !== undefined) paramRows.push([row[0], row[1]]);
    }
  }

  return (
    <div className="fixed inset-0 z-40" data-testid="lineage-panel" role="dialog" aria-modal="true">
      <button
        type="button"
        aria-label={t("common.close")}
        tabIndex={-1}
        onClick={onClose}
        className="absolute inset-0 bg-foreground/25"
      />
      <aside className="absolute inset-y-0 right-0 flex w-96 flex-col border-l bg-card shadow-[0_8px_24px_rgba(2,8,23,0.08)]">
        <div className="flex h-11 shrink-0 items-center justify-between border-b px-4">
          <h2 className="flex items-center gap-2 text-sm font-semibold">
            <Waypoints className="size-4 text-muted-foreground" />
            {t("lineage.title")}
          </h2>
          <Button variant="ghost" size="icon" aria-label={t("common.close")} data-testid="lineage-close" onClick={onClose}>
            <XIcon className="size-4" />
          </Button>
        </div>

        {loading ? (
          <div className="flex flex-1 items-center justify-center p-4">
            <p className="text-xs text-muted-foreground">{t("lineage.loading")}</p>
          </div>
        ) : !record ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
            <Waypoints className="size-10 text-muted-foreground/60" />
            <p className="text-sm font-medium text-foreground">{t("lineage.empty.title")}</p>
            <p className="max-w-64 text-xs leading-relaxed text-muted-foreground">
              {t("lineage.empty.desc")}
            </p>
          </div>
        ) : (
          <>
            <div className="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto p-4">
              <div className="flex items-center justify-between gap-2">
                {record.source === "agent-file" ? (
                  <Badge
                    className="border-transparent bg-accent text-accent-foreground"
                    data-testid="lineage-source-agent-file"
                  >
                    {t("lineage.source.agentFile")}
                  </Badge>
                ) : record.source === "engine" ? (
                  <Badge data-testid="lineage-source-engine">{t("lineage.source.engine")}</Badge>
                ) : null}
                <span className="font-mono text-[11px] text-muted-foreground">{record.at}</span>
              </div>

              <section className="flex flex-col gap-2">
                <div className="flex items-center justify-between gap-2">
                  <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("lineage.prompt")}
                  </h3>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 gap-1 px-2 text-xs"
                    data-testid="lineage-copy-prompt"
                    aria-label={t("lineage.copyPrompt")}
                    onClick={() => void copyPrompt()}
                  >
                    {copied === "prompt" ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
                    {copied === "prompt" ? t("lineage.copied") : t("lineage.copyPrompt")}
                  </Button>
                </div>
                <pre
                  data-testid="lineage-prompt"
                  className="max-h-64 overflow-y-auto whitespace-pre-wrap break-words rounded-lg bg-muted p-3 font-mono text-[11px] leading-relaxed text-foreground"
                >
                  {record.prompt}
                </pre>
              </section>

              <section className="flex flex-col gap-2">
                <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  {t("lineage.paramsTitle")}
                </h3>
                <dl className="overflow-hidden rounded-lg border" data-testid="lineage-params">
                  {paramRows.map(([key, value]) => (
                    <div
                      key={key}
                      className="flex items-baseline justify-between gap-3 border-b px-3 py-1.5 last:border-b-0"
                    >
                      <dt className="shrink-0 text-xs text-muted-foreground">
                        {t(`lineage.params.${key}`)}
                      </dt>
                      <dd className="break-all text-right font-mono text-xs text-foreground">{value}</dd>
                    </div>
                  ))}
                </dl>
              </section>
            </div>

            <div className="shrink-0 border-t p-3">
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                data-testid="lineage-copy-json"
                onClick={() => void copyJson()}
              >
                {copied === "json" ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
                {copied === "json" ? t("lineage.copied") : t("lineage.copyJson")}
              </Button>
            </div>
          </>
        )}
      </aside>
    </div>
  );
}
