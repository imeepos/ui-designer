import { useState, type ReactNode } from "react";

import { ConfirmDialog } from "@/components/ui/confirm-dialog";

export interface ConfirmRequest {
  title: string;
  description: string;
  onConfirm: () => void;
}

/** Managed destructive-confirmation dialog for panels. */
export function useConfirm(): {
  ask: (request: ConfirmRequest) => void;
  element: ReactNode;
} {
  const [request, setRequest] = useState<ConfirmRequest | null>(null);

  const element = (
    <ConfirmDialog
      open={request !== null}
      title={request?.title ?? ""}
      description={request?.description ?? ""}
      onConfirm={() => {
        request?.onConfirm();
        setRequest(null);
      }}
      onCancel={() => setRequest(null)}
    />
  );

  return { ask: setRequest, element };
}
