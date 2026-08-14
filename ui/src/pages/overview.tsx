import { SectionLabel } from "@/components/card-section";
import {
  type AttentionRow,
  AttentionSection,
} from "@/components/home/attention-section";
import { RecentActivity } from "@/components/home/recent-activity";
import { PageHeader } from "@/components/page-header";
import { StatTile } from "@/components/stat-tile";
import { Skeleton } from "@/components/ui/skeleton";
import { groupItems, heldBackCount, recentItems } from "@/lib/derive";
import { HOME_SUBTITLE, toolName } from "@/lib/labels";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

const RECENT_ACTIVITY_LIMIT = 6;

export function OverviewPage() {
  const { result } = useScanStore();
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
        <PageHeader title="Home" subtitle={HOME_SUBTITLE} />
        <div className="p-8">
          <div className="mx-auto w-full max-w-5xl space-y-6">
            <div className="space-y-3">
              <Skeleton className="h-16 w-full rounded-lg" />
              <Skeleton className="h-16 w-full rounded-lg" />
            </div>
            <div className="grid grid-cols-3 gap-3">
              <Skeleton className="h-20 rounded-lg" />
              <Skeleton className="h-20 rounded-lg" />
              <Skeleton className="h-20 rounded-lg" />
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
  const recent = recentItems(groupItems(result.items), RECENT_ACTIVITY_LIMIT);

  return (
    <div>
      <PageHeader title="Home" subtitle={HOME_SUBTITLE} />
      <div className="p-8">
        <div className="mx-auto w-full max-w-5xl space-y-6">
          <div className="space-y-3">
            <SectionLabel>Needs attention</SectionLabel>
            <AttentionSection rows={rows} />
          </div>

          <div className="space-y-3">
            <SectionLabel>Recent activity</SectionLabel>
            <RecentActivity groups={recent} />
          </div>

          <div className="space-y-3">
            <SectionLabel>At a glance</SectionLabel>
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
          </div>
        </div>
      </div>
    </div>
  );
}
