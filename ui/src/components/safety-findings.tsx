import type { Finding, ItemSafety } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { cleanSummaryLead, morePlacesLabel } from "@/lib/copy";
import { abbreviateHome } from "@/lib/drift-merge";
import { groupSkipped } from "@/lib/group-findings";
import {
  kindLabel,
  SEVERITY_BADGES,
  SEVERITY_LABELS,
  skipReasonShort,
} from "@/lib/labels";

/**
 * One finding, read top to bottom as: how bad, what it is, what to do,
 * where.
 *
 * The severity chip sits in a lane of its own rather than inline, because
 * "Serious" and "Worth a look" are different widths and an inline chip
 * starts every message at a different left edge — a column of text that
 * never lines up is the thing that makes a list of these unreadable.
 *
 * One place is named in full. The rest are counted, not listed: a rule that
 * fired in twenty files printed twenty paths, and nobody reads the
 * nineteenth. The items themselves are named under the finding, which is
 * the identification a person actually wants.
 */
export function FindingLine({
  finding,
  locations = [finding.location],
}: {
  finding: Finding;
  locations?: string[];
}) {
  return (
    <div className="flex items-start gap-3 text-[13px]">
      <div className="w-[5.5rem] shrink-0 pt-0.5">
        <Badge variant={SEVERITY_BADGES[finding.severity]}>
          {SEVERITY_LABELS[finding.severity]}
        </Badge>
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <p className="break-words">{finding.message}</p>
        <p className="break-words text-muted-foreground">
          <span className="font-medium text-foreground/70">To fix: </span>
          {finding.remediation}
        </p>
        <p
          className="truncate font-mono text-xs text-muted-foreground/80"
          title={locations.join("\n")}
        >
          {abbreviateHome(locations[0])}
          {locations.length > 1 ? (
            <span className="font-sans">
              {" "}
              {morePlacesLabel(locations.length - 1)}
            </span>
          ) : null}
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
    <p className="pt-2 text-[13px] text-muted-foreground">
      {clauses.join(" · ")}.
    </p>
  );
}
