import { useMemo, useState } from "react";
import { ChevronLeft, ChevronRight, Search } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { AnchorBadge } from "@/components/anchor-badge";
import { CompassMark } from "@/components/icons";
import { EmptyState } from "@/components/empty-state";
import { ArtImage } from "@/components/art-image";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { formatSize } from "@/lib/size";

const PAGE_SIZE = 8;

/** Home: project cards (anchor thumbnail + size + counts) with search + pagination. */
export function HomeView({ onCreate }: { onCreate: () => void }) {
  const { t } = useTranslation();
  const { state, selectProject } = useStudio();
  const [keyword, setKeyword] = useState("");
  const [page, setPage] = useState(0);

  const filtered = useMemo(() => {
    const query = keyword.trim().toLowerCase();
    if (!query) return state.projects;
    return state.projects.filter((item) => item.name.toLowerCase().includes(query));
  }, [keyword, state.projects]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount - 1);
  const visible = filtered.slice(safePage * PAGE_SIZE, (safePage + 1) * PAGE_SIZE);

  return (
    <main className="mx-auto flex w-full max-w-6xl min-w-0 flex-1 flex-col gap-4 px-6 py-6" data-testid="home-view">
      <div className="flex flex-wrap items-center gap-2">
        <div className="relative min-w-0 flex-1">
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            data-testid="project-search"
            value={keyword}
            onChange={(event) => {
              setKeyword(event.target.value);
              setPage(0);
            }}
            placeholder={t("home.searchPlaceholder")}
            className="pl-8"
          />
        </div>
        <Button size="sm" data-testid="new-project" onClick={onCreate}>
          {t("home.newProject")}
        </Button>
      </div>

      {state.projects.length === 0 ? (
        <EmptyState
          mark="boat"
          title={t("home.empty.title")}
          description={t("home.empty.desc")}
          action={
            <Button size="sm" data-testid="home-empty-cta" onClick={onCreate}>
              {t("home.newProject")}
            </Button>
          }
        />
      ) : filtered.length === 0 ? (
        <p className="py-16 text-center text-sm text-muted-foreground">{t("home.searchEmpty")}</p>
      ) : (
        <>
          <div
            className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4"
            data-testid="project-grid"
          >
            {visible.map((item) => (
              <button
                key={item.id}
                type="button"
                data-testid={`project-card-${item.id}`}
                onClick={() => void selectProject(item.id)}
                className="flex flex-col overflow-hidden rounded-lg border bg-card text-left transition-all duration-150 ease-out hover:-translate-y-0.5 hover:border-primary/60 hover:shadow-[0_8px_24px_rgba(2,8,23,0.08)]"
              >
                <span
                  className="relative block aspect-[16/10] w-full overflow-hidden border-b bg-muted"
                  data-testid={`project-thumb-${item.id}`}
                >
                  {item.anchorUrl ? (
                    <ArtImage
                      src={item.anchorUrl}
                      alt={t("home.anchorThumbAlt", { name: item.name })}
                      className="absolute inset-0"
                    />
                  ) : (
                    <span className="absolute inset-0 flex flex-col items-center justify-center gap-2">
                      <CompassMark className="size-14 text-primary" />
                      <span className="text-[11px] text-muted-foreground">
                        {t("home.noAnchorThumb")}
                      </span>
                    </span>
                  )}
                </span>
                <span className="flex flex-col gap-1.5 p-4">
                  <span className="flex min-w-0 items-center gap-2">
                    <span className="min-w-0 flex-1 truncate text-sm font-semibold text-foreground">
                      {item.name}
                    </span>
                    {item.hasAnchor && <AnchorBadge />}
                  </span>
                  <span className="flex items-center gap-2 font-mono text-[11px] text-muted-foreground">
                    <span data-testid={`project-size-${item.id}`}>{formatSize(item.size)}</span>
                    <span aria-hidden="true">·</span>
                    <span>{t("home.pageCount", { count: item.pageCount })}</span>
                    <span aria-hidden="true">·</span>
                    <span>{t("home.componentCount", { count: item.componentCount })}</span>
                  </span>
                  <span className="font-mono text-[11px] text-muted-foreground">
                    {t("home.created", {
                      date: new Date(item.createdAt).toLocaleDateString(),
                    })}
                  </span>
                </span>
              </button>
            ))}
          </div>
          <div className="flex items-center justify-end gap-2" data-testid="pagination">
            <Button
              variant="outline"
              size="sm"
              data-testid="pagination-prev"
              disabled={safePage === 0}
              onClick={() => setPage((prev) => Math.max(0, prev - 1))}
              aria-label={t("home.prevPage")}
            >
              <ChevronLeft className="size-3.5" />
            </Button>
            <span className="font-mono text-xs text-muted-foreground">
              {safePage + 1} / {pageCount}
            </span>
            <Button
              variant="outline"
              size="sm"
              data-testid="pagination-next"
              disabled={safePage >= pageCount - 1}
              onClick={() => setPage((prev) => Math.min(pageCount - 1, prev + 1))}
              aria-label={t("home.nextPage")}
            >
              <ChevronRight className="size-3.5" />
            </Button>
          </div>
        </>
      )}
    </main>
  );
}
