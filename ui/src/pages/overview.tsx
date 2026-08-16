import {
  type AttentionRow,
  AttentionSection,
} from "@/components/home/attention-section";
import { RecentActivity } from "@/components/home/recent-activity";
import { PageHeader } from "@/components/page-header";
import { Section } from "@/components/section";
import { StatTile } from "@/components/stat-tile";
import { Skeleton } from "@/components/ui/skeleton";
import {
  FORKED_ATTENTION_DETAIL,
  forkedAttentionTitle,
  HOME_SUBTITLE,
  RECENTLY_CHANGED_HELP,
  REVIEW_ACTION_LABEL,
} from "@/lib/copy";
import { groupItems, heldBackCount, recentItems } from "@/lib/derive";
import { toolName } from "@/lib/labels";
import { CONTENT_WIDTH, PAGE_BODY } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";
import { useUpdatesStore } from "@/stores/updates";

const RECENT_ACTIVITY_LIMIT = 6;

export function OverviewPage() {
  const { result } = useScanStore();
  const views = useAuditStore((s) => s.views);
  const projectCount = useSettingsStore(
    (s) => s.settings?.projects?.length ?? 0,
  );
  const setPage = useNavStore((s) => s.setPage);
  const goToPackage = useNavStore((s) => s.goToPackage);
  const updateRows = useUpdatesStore((s) => s.rows);
  const editedPackages = updateRows.filter((row) => row.blockedByLocalEdit);
  const goToTools = useNavStore((s) => s.goToTools);
  const goToLibrary = useNavStore((s) => s.goToLibrary);

  if (!result) {
    return (
      <div>
        <PageHeader title="Home" subtitle={HOME_SUBTITLE} />
        <div className={PAGE_BODY}>
          <div className={cn("space-y-6", CONTENT_WIDTH)}>
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
  if (editedPackages.length > 0) {
    const first = editedPackages[0];
    rows.push({
      key: "edited",
      tone: "warning",
      title: forkedAttentionTitle(editedPackages.length),
      detail: FORKED_ATTENTION_DETAIL,
      action:
        editedPackages.length === 1 && first
          ? {
              label: `Open ${first.name}`,
              onClick: () =>
                goToPackage({
                  kind: first.kind,
                  name: first.name,
                  scope: first.scope,
                }),
            }
          : { label: "Open Library", onClick: () => setPage("library") },
    });
  }
  if (blocked > 0) {
    rows.push({
      key: "safety",
      tone: "critical",
      title:
        blocked === 1
          ? "1 serious problem found"
          : `${blocked} serious problems found`,
      detail:
        "They're on your machine now. Read what was found before vstack installs or updates them.",
      action: { label: REVIEW_ACTION_LABEL, onClick: () => setPage("review") },
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
      detail: "Nothing is written until you apply them.",
      action: { label: REVIEW_ACTION_LABEL, onClick: () => setPage("review") },
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
      detail: "Already on your machine, but vstack didn't put them there.",
      action: { label: REVIEW_ACTION_LABEL, onClick: () => setPage("review") },
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
      <div className={PAGE_BODY}>
        <div className={cn("flex flex-col gap-10", CONTENT_WIDTH)}>
          <Section title="Needs attention">
            <AttentionSection rows={rows} />
          </Section>

          <Section title="Recently changed" description={RECENTLY_CHANGED_HELP}>
            <RecentActivity groups={recent} />
          </Section>

          <Section title="At a glance">
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
          </Section>
        </div>
      </div>
    </div>
  );
}
