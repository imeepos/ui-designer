import { Fragment } from "react";
import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";

export const STEP_IDS = ["project", "board", "page", "component"] as const;

export type StepId = (typeof STEP_IDS)[number];
export type StepState = "complete" | "current" | "upcoming";

// Phase 1 renders static states: `current` is caller-provided; Phase 3 wires real progress
const PREREQUISITES: Record<StepId, StepId[]> = {
  project: [],
  board: ["project"],
  page: ["project", "board"],
  component: ["project", "board", "page"],
};

export function stepState(id: StepId, current: StepId): StepState {
  if (id === current) return "current";
  return PREREQUISITES[current].includes(id) ? "complete" : "upcoming";
}

type StepperProps = {
  current: StepId;
  className?: string;
};

export function Stepper({ current, className }: StepperProps) {
  const { t } = useTranslation();

  return (
    <nav
      aria-label={t("steps.label")}
      data-testid="stepper"
      className={cn("flex shrink-0 items-center gap-2 border-b px-4 py-2", className)}
    >
      {STEP_IDS.map((id, index) => {
        const state = stepState(id, current);
        const prevState = index > 0 ? stepState(STEP_IDS[index - 1], current) : null;
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
            <div
              data-testid={`step-${id}`}
              data-state={state}
              className="flex items-center gap-2"
            >
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
                <span
                  className={cn(
                    "hidden text-[10px] tracking-wide text-muted-foreground sm:block",
                  )}
                >
                  {t(`steps.${id}.desc`)}
                </span>
              </span>
            </div>
          </Fragment>
        );
      })}
    </nav>
  );
}
