import { CheckCircle2 } from "lucide-react";
import { PageHeader } from "@/components/page-header";
import { StatTile } from "@/components/stat-tile";
import { StatusDot } from "@/components/status-dot";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { heldBackCount } from "@/lib/derive";
import { toolName } from "@/lib/labels";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

interface AttentionRow {
  key: string;
  tone: "critical" | "warning" | "info" | "muted";
  title: string;
  detail: string;
  action?: { label: string; onClick: () => void };
}

function AttentionCard({
  row,
  primary,
}: {
  row: AttentionRow;
  primary: boolean;
}) {
  return (
    <Card>
      <CardContent className="flex items-center justify-between gap-4">
        <div className="flex items-start gap-3">
          <StatusDot tone={row.tone} className="mt-2" />
          <div>
            <p className="font-medium">{row.title}</p>
            <p className="text-sm text-muted-foreground">{row.detail}</p>
          </div>
        </div>
        {row.action ? (
          <Button
            variant={primary ? "default" : "outline"}
            className="shrink-0"
            onClick={row.action.onClick}
          >
            {row.action.label}
          </Button>
        ) : null}
      </CardContent>
    </Card>
  );
}

export function OverviewPage() {
  const { result, error } = useScanStore();
  const views = useAuditStore((s) => s.views);
  const projectCount = useSettingsStore(
    (s) => s.settings?.projects?.length ?? 0,
  );
  const setPage = useNavStore((s) => s.setPage);
  const goToTools = useNavStore((s) => s.goToTools);
  const goToLibrary = useNavStore((s) => s.goToLibrary);

  if (!result) {
    return (
      <div>
        <PageHeader title="Home" subtitle="What needs your attention" />
        <div className="p-8">
          <div className="mx-auto w-full max-w-5xl space-y-6">
            <div className="grid grid-cols-3 gap-3">
              <Skeleton className="h-20 rounded-lg" />
              <Skeleton className="h-20 rounded-lg" />
              <Skeleton className="h-20 rounded-lg" />
            </div>
            <div className="space-y-3">
              <Skeleton className="h-16 w-full rounded-lg" />
              <Skeleton className="h-16 w-full rounded-lg" />
            </div>
          </div>
        </div>
      </div>
    );
  }

  const allDrift = views.flatMap((view) => view.drift);
  const actionableCount = allDrift.filter(
    (d) => d.state !== "unmanaged",
  ).length;
  const unmanagedCount = allDrift.filter((d) => d.state === "unmanaged").length;
  const blocked = heldBackCount(views);
  const missing = result.missingProjects;

  const rows: AttentionRow[] = [];
  if (blocked > 0) {
    rows.push({
      key: "safety",
      tone: "critical",
      title:
        blocked === 1
          ? "1 install held back for safety"
          : `${blocked} installs held back for safety`,
      detail: "Findings need a look before these can install.",
      action: { label: "Review findings", onClick: () => setPage("review") },
    });
  }
  if (actionableCount > 0) {
    rows.push({
      key: "drift",
      tone: "info",
      title:
        actionableCount === 1
          ? "1 change ready to apply"
          : `${actionableCount} changes ready to apply`,
      detail: "Review them before anything touches your files.",
      action: { label: "Review changes", onClick: () => setPage("review") },
    });
  }
  if (unmanagedCount > 0) {
    rows.push({
      key: "unmanaged",
      tone: "muted",
      title:
        unmanagedCount === 1
          ? "1 item isn't managed yet"
          : `${unmanagedCount} items aren't managed yet`,
      detail:
        "Found on this computer — manage them to keep them updated and checked.",
      action: { label: "Have a look", onClick: () => setPage("review") },
    });
  }
  if (missing.length > 0) {
    rows.push({
      key: "missing-projects",
      tone: "warning",
      title:
        missing.length === 1
          ? "1 project folder can't be found"
          : `${missing.length} project folders can't be found`,
      detail:
        missing.length === 1
          ? `We can't find ${missing[0]}. If you moved it, add it again.`
          : "If you moved these, add them again from Tools & Projects.",
      action: {
        label: "Open Projects",
        onClick: () => goToTools("projects"),
      },
    });
  }
  if (result.warnings.length > 0) {
    rows.push({
      key: "warnings",
      tone: "warning",
      title:
        result.warnings.length === 1
          ? "1 file couldn't be read"
          : `${result.warnings.length} files couldn't be read`,
      detail: result.warnings[0],
    });
  }

  const toolNames = result.harnesses.map((h) => toolName(h.harness)).join(", ");

  return (
    <div>
      <PageHeader title="Home" subtitle="What needs your attention" />
      <div className="p-8">
        <div className="mx-auto w-full max-w-5xl space-y-6">
          {error ? <p className="text-sm text-destructive">{error}</p> : null}

          <div className="grid grid-cols-3 gap-3">
            <StatTile
              label="Tools"
              value={result.harnesses.length}
              detail={toolNames || undefined}
              onClick={() => goToTools("tools")}
            />
            <StatTile
              label="Installed"
              value={result.items.length}
              onClick={() => goToLibrary()}
            />
            <StatTile
              label="Projects"
              value={projectCount}
              onClick={() => goToTools("projects")}
            />
          </div>

          {rows.length === 0 ? (
            <div className="flex items-center gap-3 rounded-lg border bg-muted/30 px-5 py-4">
              <CheckCircle2 className="size-5 text-good" />
              <div>
                <p className="font-medium">You're all caught up.</p>
                <p className="text-sm text-muted-foreground">
                  Everything matches what you've chosen to install.
                </p>
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              {rows.map((row, i) => (
                <AttentionCard key={row.key} row={row} primary={i === 0} />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
