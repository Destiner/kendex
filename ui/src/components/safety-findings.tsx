import type { Finding, ItemSafety } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { groupSkipped } from "@/lib/group-findings";
import {
  cleanSummaryLead,
  kindLabel,
  SEVERITY_BADGES,
  SEVERITY_LABELS,
  skipReasonShort,
} from "@/lib/labels";

// The badge anchors a fixed-width column so every finding's message lines
// up regardless of severity label length; the fix line sits visually
// quieter than the message so a scan of the stack reads problem, then
// remedy, without the two competing for attention.
export function FindingLine({ finding }: { finding: Finding }) {
  return (
    <div className="flex items-start gap-2 text-xs">
      <Badge
        variant={SEVERITY_BADGES[finding.severity]}
        className="mt-0.5 shrink-0"
      >
        {SEVERITY_LABELS[finding.severity]}
      </Badge>
      <div className="min-w-0 flex-1 space-y-0.5">
        <p className="break-words text-muted-foreground">{finding.message}</p>
        <p className="break-all font-mono text-muted-foreground">
          {finding.location}
        </p>
        <p className="break-words text-muted-foreground/70">
          <span className="text-muted-foreground/50">↳ Fix: </span>
          {finding.remediation}
        </p>
      </div>
    </div>
  );
}

// One quiet line under the safety list — clean items don't get a row of
// their own, just a tally, so a scan of the section ends on reassurance
// instead of trailing off after the last warning.
export function SafetyCleanSummary({ rows }: { rows: ItemSafety[] }) {
  if (rows.length === 0) return null;
  const clauses = [
    cleanSummaryLead(rows.length),
    ...groupSkipped(rows).map((group) => {
      const noun = group.kind
        ? kindLabel(group.kind, group.count).toLowerCase()
        : `item${group.count === 1 ? "" : "s"}`;
      return `${group.count} ${noun} ${skipReasonShort(group.reason)}`;
    }),
  ];
  return (
    <p className="text-xs text-muted-foreground">{clauses.join(" · ")}.</p>
  );
}
