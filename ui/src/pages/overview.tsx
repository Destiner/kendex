import { Boxes, RefreshCw, TriangleAlert } from "lucide-react";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { countByKind } from "@/lib/derive";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";

export function OverviewPage() {
  const { result, scanning, error, refresh } = useScanStore();
  const setPage = useNavStore((s) => s.setPage);

  if (!result) {
    return (
      <div className="p-8">
        <Skeleton className="h-24 w-full" />
      </div>
    );
  }
  const counts = countByKind(result.items);

  return (
    <div>
      <PageHeader
        title="Overview"
        subtitle="What this machine runs, at a glance"
      />
      <div className="space-y-6 p-8">
        {error ? <p className="text-sm text-destructive">{error}</p> : null}

        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm text-muted-foreground">
                Harnesses detected
              </CardTitle>
            </CardHeader>
            <CardContent className="text-3xl font-semibold">
              {result.harnesses.length}
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle className="text-sm text-muted-foreground">
                Items observed
              </CardTitle>
            </CardHeader>
            <CardContent className="text-3xl font-semibold">
              {result.items.length}
            </CardContent>
          </Card>
          <Card className="col-span-2">
            <CardHeader>
              <CardTitle className="text-sm text-muted-foreground">
                By kind
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-wrap gap-2">
              {counts.size === 0 ? (
                <span className="text-sm text-muted-foreground">
                  nothing observed yet
                </span>
              ) : (
                [...counts.entries()].map(([kind, count]) => (
                  <Badge key={kind} variant="secondary">
                    {kind} {count}
                  </Badge>
                ))
              )}
            </CardContent>
          </Card>
        </div>

        {result.missingProjects.length > 0 ? (
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <TriangleAlert className="size-4" /> Missing projects
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-1 text-sm text-muted-foreground">
              {result.missingProjects.map((p) => (
                <p key={p}>{p} — registered, but the directory is gone</p>
              ))}
            </CardContent>
          </Card>
        ) : null}

        {result.warnings.length > 0 ? (
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Scan warnings</CardTitle>
            </CardHeader>
            <CardContent className="space-y-1 text-sm text-muted-foreground">
              {result.warnings.map((w) => (
                <p key={w}>{w}</p>
              ))}
            </CardContent>
          </Card>
        ) : null}

        <div className="flex gap-2">
          <Button onClick={() => void refresh()} disabled={scanning}>
            <RefreshCw className="size-4" /> Rescan
          </Button>
          <Button variant="outline" onClick={() => setPage("items")}>
            <Boxes className="size-4" /> Browse items
          </Button>
        </div>
      </div>
    </div>
  );
}
