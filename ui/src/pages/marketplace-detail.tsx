import { MoreHorizontal, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { AboutSection } from "@/components/marketplaces/about-section";
import { BundleCards } from "@/components/marketplaces/bundle-cards";
import { PackagesTable } from "@/components/marketplaces/packages-table";
import { UnsubscribeDialog } from "@/components/marketplaces/unsubscribe-dialog";
import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { PAGE_BODY, PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { marketKey, useMarketplacesStore } from "@/stores/marketplaces";
import { useNavStore } from "@/stores/nav";

/** One subscription's own page: what it offers and what it says about
 * itself. Nested under Marketplaces — the breadcrumb strip above carries
 * the way back. */
export function MarketplaceDetailPage() {
  const marketplaceRef = useNavStore((s) => s.marketplaceRef);
  const rows = useMarketplacesStore((s) => s.rows);
  const packages = useMarketplacesStore((s) => s.packages);
  const load = useMarketplacesStore((s) => s.load);
  const loadPackages = useMarketplacesStore((s) => s.loadPackages);
  const toggle = useMarketplacesStore((s) => s.toggle);
  const checkForUpdates = useMarketplacesStore((s) => s.checkForUpdates);
  const busy = useMarketplacesStore((s) => s.busy);
  const [unsubscribeOpen, setUnsubscribeOpen] = useState(false);

  const key = marketplaceRef
    ? marketKey(marketplaceRef.scope, marketplaceRef.source)
    : null;
  const row = marketplaceRef
    ? rows.find(
        (r) =>
          r.name === marketplaceRef.source &&
          marketKey(r.scope, r.name) === key,
      )
    : undefined;
  const offered = key ? (packages[key] ?? []) : [];

  useEffect(() => {
    void load();
  }, [load]);
  useEffect(() => {
    if (!marketplaceRef) return;
    void loadPackages(marketplaceRef.scope, marketplaceRef.source);
  }, [marketplaceRef, loadPackages]);

  if (!marketplaceRef) return null;
  const { scope, source } = marketplaceRef;
  const meta = row?.meta;
  const metaLine = [
    row?.repo ?? row?.path,
    row?.commit ? `@ ${row.commit.slice(0, 7)}` : null,
    meta?.license,
    meta?.author ? `by ${meta.author}` : null,
  ].filter(Boolean);

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        wide
        title={source}
        subtitle={
          <>
            {meta?.description ? <p>{meta.description}</p> : null}
            {metaLine.length > 0 ? (
              <p className="mt-1 font-mono text-xs">{metaLine.join(" · ")}</p>
            ) : null}
          </>
        }
        action={
          <>
            {row ? (
              <Switch
                checked={row.enabled}
                onCheckedChange={(enabled) =>
                  void toggle(scope, source, enabled)
                }
                aria-label={row.enabled ? "Turn off" : "Turn on"}
              />
            ) : null}
            <Button
              size="sm"
              variant="outline"
              disabled={busy}
              onClick={() => void checkForUpdates()}
            >
              <RefreshCw className={cn("size-4", busy && "animate-spin")} />
              Check for updates
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    size="icon-xs"
                    variant="quiet"
                    aria-label="More actions"
                  >
                    <MoreHorizontal className="size-4" />
                  </Button>
                }
              />
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  className="text-critical"
                  onClick={() => setUnsubscribeOpen(true)}
                >
                  Unsubscribe…
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </>
        }
      />
      <UnsubscribeDialog
        open={unsubscribeOpen}
        onOpenChange={setUnsubscribeOpen}
        scope={scope}
        source={source}
      />
      <Tabs
        defaultValue="bundles"
        className="flex min-h-0 flex-1 flex-col gap-0"
      >
        <div className={cn("pb-3", PAGE_GUTTER)}>
          <div className={WIDE_CONTENT_WIDTH}>
            <TabsList>
              <TabsTrigger value="bundles">Bundles</TabsTrigger>
              <TabsTrigger value="packages">Packages</TabsTrigger>
              <TabsTrigger value="about">About</TabsTrigger>
            </TabsList>
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          <div className={cn(PAGE_BODY, "pt-0")}>
            <div className={WIDE_CONTENT_WIDTH}>
              <TabsContent value="bundles">
                <BundleCards scope={scope} source={source} offered={offered} />
              </TabsContent>
              <TabsContent value="packages">
                {offered.length === 0 ? (
                  <p className="py-16 text-center text-sm text-muted-foreground">
                    Nothing to list yet — this marketplace hasn't been fetched,
                    or offers no packages.
                  </p>
                ) : (
                  <PackagesTable
                    entries={offered.map((pkg) => ({
                      scope,
                      source,
                      row: pkg,
                    }))}
                    showMarketplace={false}
                  />
                )}
              </TabsContent>
              <TabsContent value="about">
                <AboutSection
                  scope={scope}
                  source={source}
                  meta={meta ?? null}
                />
              </TabsContent>
            </div>
          </div>
        </div>
      </Tabs>
    </div>
  );
}
