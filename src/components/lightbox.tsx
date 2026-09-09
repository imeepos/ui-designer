import { useTranslation } from "react-i18next";

import { ArtImage } from "@/components/art-image";
import { Modal } from "@/components/ui/modal";

export type LightboxProps = {
  open: boolean;
  onClose: () => void;
  imageUrl: string;
  filter?: string;
  alt: string;
  caption?: string;
};

/** Enlarged preview (THEME §5 hover action). */
export function Lightbox({ open, onClose, imageUrl, filter, alt, caption }: LightboxProps) {
  const { t } = useTranslation();

  return (
    <Modal open={open} onClose={onClose} title={caption ?? alt} wide testId="lightbox">
      <div className="flex flex-col gap-2">
        <ArtImage
          src={imageUrl}
          filter={filter}
          alt={alt}
          fit="contain"
          className="max-h-[70vh] min-h-64 w-full rounded-md border"
        />
        <p className="font-mono text-[11px] text-muted-foreground">{t("lightbox.hint")}</p>
      </div>
    </Modal>
  );
}
