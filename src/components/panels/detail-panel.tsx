import { useTranslation } from "react-i18next";

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

/** Right column: details & actions (320px, THEME §4) */
export function DetailPanel() {
  const { t } = useTranslation();

  return (
    <aside
      data-testid="detail-panel"
      className="flex w-80 shrink-0 flex-col border-l"
    >
      <div className="flex h-10 shrink-0 items-center px-4">
        <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("panel.detail.title")}
        </h2>
      </div>
      <div className="flex min-h-0 flex-1 flex-col p-4 pt-0">
        <Card data-testid="detail-placeholder">
          <CardHeader>
            <CardTitle>{t("panel.detail.placeholder.title")}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-muted-foreground">
              {t("panel.detail.empty")}
            </p>
          </CardContent>
        </Card>
      </div>
    </aside>
  );
}
