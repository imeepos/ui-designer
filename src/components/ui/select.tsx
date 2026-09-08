import * as React from "react";
import { ChevronDown } from "lucide-react";

import { cn } from "@/lib/utils";

/** Native select styled to the rudder theme (no extra dependency). */
function Select({ className, children, ...props }: React.ComponentProps<"select">) {
  return (
    <div className="relative flex w-full items-center">
      <select
        data-slot="select"
        className={cn(
          "h-8 w-full appearance-none rounded-md border bg-background px-2.5 pr-7 text-sm transition-colors duration-150 ease-out outline-none",
          "focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30",
          "disabled:cursor-not-allowed disabled:opacity-50",
          className,
        )}
        {...props}
      >
        {children}
      </select>
      <ChevronDown
        aria-hidden="true"
        className="pointer-events-none absolute right-2 size-3.5 text-muted-foreground"
      />
    </div>
  );
}

export { Select };
