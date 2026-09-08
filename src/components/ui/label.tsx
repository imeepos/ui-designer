import * as React from "react";

import { cn } from "@/lib/utils";

/** THEME §3: labels are small, tracking-wide. */
function Label({ className, ...props }: React.ComponentProps<"label">) {
  return (
    <label
      data-slot="label"
      className={cn(
        "flex items-center gap-1.5 text-xs font-medium tracking-wide text-foreground select-none",
        className,
      )}
      {...props}
    />
  );
}

export { Label };
