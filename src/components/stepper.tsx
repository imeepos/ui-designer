import { Fragment } from "react";
import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ProjectDetail } from "@/lib/api/types";
import { cn } from "@/lib/utils";

export const STEP_IDS = ["project", "board", "page", "component"] as const;

export type StepId = (typeof STEP_IDS)[number];
export type StepState = "complete" | "current" | "upcoming";

/** Whether a step may be visited: prerequisites must be satisfied (PRD §3). */
export function stepUnlockedMap(project: ProjectDetail | null): Record<StepId, boolean> {
  return {
    project: true,
    board: project !== null,
    page: project?.anchor != null,
    component:
      project?.anchor != null && project.pages.some((page) => page.current !== null),
  };
}

/** Visual state: viewed step wins, done steps show the primary check. */
export function computeStepStates(
  project: ProjectDetail | null,
  view: StepId,
): Record<StepId, StepState> {
  const done: Record<StepId, boolean> = {
    project: project !== null,
    board: project?.anchor != null,
    page: project != null && project.pages.some((page) => page.current !== null),
    component:
      project != null && project.components.some((component) => component.current !== null),
  };
  const states = {} as Record<StepId, StepState>;
  for (const id of STEP_IDS) {
    states[id] = view === id ? "current" : done[id] ? "complete" : "upcoming";
  }
  return states;
}

type StepperProps = {
  current: StepId;
  states: Record<StepId, StepState>;
  unlocked: Record<StepId, boolean>;
  onSelect?: (id: StepId) => void;
};

/** THEME §5: complete = primary check, current = brass dot, upcoming = muted. */
export function Stepper({ current, states, unlocked, onSelect }: StepperProps) {
  const { t } = useTranslation();

  return (
    <nav
      aria-label={t("steps.label")}
      data-testid="stepper"
      className="flex shrink-0 items-center gap-2 border-b px-4 py-2"
    >
      {STEP_IDS.map((id, index) => {
        const state = states[id];
        const prevState = index > 0 ? states[STEP_IDS[index - 1]] : null;
        const locked = !unlocked[id];
        const clickable = onSelect != null && !locked;
        return (
          <Fragment key={id}>
            {index > 0 && (
              <span
                aria-hidden="true"
                className={cn(
                  "h-px w-10 shrink-0",
                  prevState === "complete" ? "bg-primary" : "bg-border",
                )}
              />
            )}
            <button
              type="button"
              disabled={!clickable}
              aria-current={state === "current" ? "step" : undefined}
              title={locked ? t("steps.locked") : t(`steps.${id}.title`)}
              onClick={() => onSelect?.(id)}
              data-testid={`step-nav-${id}`}
              className="flex items-center gap-2 rounded-sm disabled:cursor-default"
            >
              <div data-testid={`step-${id}`} data-state={state} className="flex items-center gap-2">
                <span
                  className={cn(
                    "flex size-6 shrink-0 items-center justify-center rounded-full border text-xs transition-colors duration-150 ease-out",
                    state === "complete" &&
                      "border-primary bg-primary text-primary-foreground",
                    state === "current" && "border-accent bg-background",
                    state === "upcoming" && "border-transparent bg-muted text-muted-foreground",
                  )}
                >
                  {state === "complete" ? (
                    <Check className="size-3.5" />
                  ) : state === "current" ? (
                    <span aria-hidden="true" className="size-2 rounded-full bg-accent" />
                  ) : (
                    <span aria-hidden="true" className="size-1 rounded-full bg-muted-foreground" />
                  )}
                </span>
                <span className="flex flex-col leading-tight">
                  <span
                    className={cn(
                      "text-xs",
                      state === "current" && "text-sm font-semibold text-foreground",
                      state === "complete" && "text-foreground",
                      state === "upcoming" && "text-muted-foreground",
                    )}
                  >
                    {t(`steps.${id}.title`)}
                  </span>
                  <span className="hidden text-[11px] tracking-wide text-muted-foreground sm:block">
                    {t(`steps.${id}.desc`)}
                  </span>
                </span>
              </div>
            </button>
          </Fragment>
        );
      })}
      <span className="sr-only">{current}</span>
    </nav>
  );
}
