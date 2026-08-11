import { CheckCircle2, Library, TriangleAlert } from "lucide-react";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { countByKind } from "@/lib/derive";
import { kindLabel, toolName } from "@/lib/labels";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

function HeroStatus({
  driftCount,
  onReview,
}: {
  driftCount: number;
  onReview: () => void;
}) {
  if (driftCount === 0) {
    return (
      <div className="flex items-center gap-3 rounded-lg border bg-muted/30 px-5 py-4">
        <CheckCircle2 className="size-5 text-primary" />
        <div>
          <p className="font-medium">Everything is in sync.</p>
          <p className="text-sm text-muted-foreground">
            Your tools match what you've chosen to install.
          </p>
        </div>
      </div>
    );
  }
  return (
    <div className="flex items-center justify-between gap-4 rounded-lg border bg-muted/30 px-5 py-4">
      <div className="flex items-center gap-3">
        <TriangleAlert className="size-5 text-muted-foreground" />
        <div>
          <p className="font-medium">
            {driftCount === 1
              ? "1 thing needs attention"
              : `${driftCount} things need attention`}
          </p>
          <p className="text-sm text-muted-foreground">
            Some items are out of sync with what you've set up.
          </p>
        </div>
      </div>
      <Button onClick={onReview}>Review and fix</Button>
    </div>
  );
}

export function OverviewPage() {
  const { result, error } = useScanStore();
  const driftCount = useAuditStore((s) =>
    s.views.reduce((sum, view) => sum + view.drift.length, 0),
  );
  const projectCount = useSettingsStore(
    (s) => s.settings?.projects?.length ?? 0,
  );
  const setPage = useNavStore((s) => s.setPage);

  if (!result) {
    return (
      <div className="p-8">
        <Skeleton className="h-24 w-full" />
      </div>
    );
  }
  const counts = countByKind(result.items);
  const toolNames = result.harnesses.map((h) => toolName(h.harness)).join(", ");

  return (
    <div>
      <PageHeader title="Home" subtitle="What's set up on this machine" />
      <div className="space-y-6 p-8">
        {error ? <p className="text-sm text-destructive">{error}</p> : null}

        <HeroStatus driftCount={driftCount} onReview={() => setPage("audit")} />

        <div className="grid grid-cols-2 gap-4 lg:grid-cols-5">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm text-muted-foreground">
                Tools
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-3xl font-semibold">
                {result.harnesses.length}
              </p>
              <p className="mt-1 text-xs text-muted-foreground">
                {toolNames || "None detected"}
              </p>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle className="text-sm text-muted-foreground">
                Installed
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-3xl font-semibold">{result.items.length}</p>
              <p className="mt-1 text-xs text-muted-foreground">
                across all projects
              </p>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle className="text-sm text-muted-foreground">
                Projects
              </CardTitle>
            </CardHeader>
            <CardContent className="text-3xl font-semibold">
              {projectCount}
            </CardContent>
          </Card>
          <Card className="col-span-2">
            <CardHeader>
              <CardTitle className="text-sm text-muted-foreground">
                By type
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-wrap gap-2">
              {counts.size === 0 ? (
                <span className="text-sm text-muted-foreground">
                  Nothing here yet.
                </span>
              ) : (
                [...counts.entries()].map(([kind, count]) => (
                  <Badge key={kind} variant="secondary">
                    {count} {kindLabel(kind, count)}
                  </Badge>
                ))
              )}
            </CardContent>
          </Card>
        </div>

        <Button variant="ghost" onClick={() => setPage("items")}>
          <Library className="size-4" /> Browse library
        </Button>

        {result.missingProjects.length > 0 ? (
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <TriangleAlert className="size-4" /> Project folder not found
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-1 text-sm text-muted-foreground">
              {result.missingProjects.map((p) => (
                <p key={p}>
                  We can't find {p}. If you moved it, add it again from
                  Projects.
                </p>
              ))}
            </CardContent>
          </Card>
        ) : null}

        {result.warnings.length > 0 ? (
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">
                Some things couldn't be read
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-1 text-sm text-muted-foreground">
              {result.warnings.map((w) => (
                <p key={w}>{w}</p>
              ))}
            </CardContent>
          </Card>
        ) : null}
      </div>
    </div>
  );
}
