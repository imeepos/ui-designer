import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Modal } from "@/components/ui/modal";

export type ConfirmDialogProps = {
  open: boolean;
  title: string;
  description: string;
  onConfirm: () => void;
  onCancel: () => void;
  testId?: string;
};

/** Destructive confirmation (delete artifacts). */
export function ConfirmDialog({
  open,
  title,
  description,
  onConfirm,
  onCancel,
  testId = "confirm-dialog",
}: ConfirmDialogProps) {
  const { t } = useTranslation();

  return (
    <Modal
      open={open}
      onClose={onCancel}
      title={title}
      testId={testId}
      footer={
        <>
          <Button variant="outline" size="sm" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
          <Button variant="destructive" size="sm" data-testid="confirm-accept" onClick={onConfirm}>
            {t("common.delete")}
          </Button>
        </>
      }
    >
      <p className="text-xs leading-relaxed text-muted-foreground">{description}</p>
    </Modal>
  );
}
