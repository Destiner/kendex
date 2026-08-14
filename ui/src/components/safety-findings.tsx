import type { Finding, ItemSafety } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { Badge } from "@/components/ui/badge";
import { heldBack } from "@/lib/derive";
import { groupSkipped } from "@/lib/group-findings";
import {
  hookDisplayName,
  kindLabel,
  SEVERITY_BADGES,
  SEVERITY_LABELS,
  skipReasonShort,
  toolName,
  VERDICT_BADGES,
  VERDICT_LABELS,
} from "@/lib/labels";

// A hook's raw identifier ("PreToolUse:*:claude-hook") is never the title —
// the trailing name is, with the full identifier kept in mono beneath it.
// Shared with the affected-set groups in safety-findings-affected.tsx.
export function ItemTitle({
  row,
}: {
  row: {
    kind: ItemSafety["kind"];
    name: string;
    harness: ItemSafety["harness"];
  };
}) {
  const name = row.kind === "hook" ? hookDisplayName(row.name) : row.name;
  return (
    <span className="min-w-0">
      <span className="text-sm font-medium">{name}</span>{" "}
      <span className="text-xs text-muted-foreground">
        {kindLabel(row.kind)} · {toolName(row.harness)}
      </span>
      {row.kind === "hook" ? (
        <span className="block break-all font-mono text-xs text-muted-foreground">
          {row.name}
        </span>
      ) : null}
    </span>
  );
}

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

// A row where every rule was skipped has not been audited, and showing
// nothing would read as an audit that passed. It gets a line of its own
// saying what could not be looked at.
function NotChecked({ row }: { row: ItemSafety }) {
  if (row.skipped.length === 0) return null;
  return (
    <p className="text-xs text-muted-foreground">
      Not fully checked here: {row.skipped.length} rule
      {row.skipped.length === 1 ? "" : "s"} had nothing to read —{" "}
      {row.skipped[0].reason}
    </p>
  );
}

function BlockedItem({ row }: { row: ItemSafety }) {
  return (
    <div className="space-y-1 text-sm">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant={heldBack(row) ? VERDICT_BADGES.block : "warning"}>
          {heldBack(row) ? VERDICT_LABELS.block : "Accepted by you"}
        </Badge>
        <ItemTitle row={row} />
      </div>
      {row.override.state === "stale" ? (
        <p className="text-xs text-muted-foreground">
          You accepted this before, but {row.override.why}.
        </p>
      ) : null}
      {row.override.state === "active" ? (
        <p className="text-xs text-muted-foreground">
          You read these findings and accepted them, so this stays installed.
        </p>
      ) : null}
      <NotChecked row={row} />
      {row.findings.map((finding) => (
        <FindingLine
          key={`${finding.rule}:${finding.location}:${finding.message}`}
          finding={finding}
        />
      ))}
    </div>
  );
}

// One export per verdict, so the page can place each at the urgency it
// earns — held-back items loudest near the top of the card, warnings from
// safety-findings-affected.tsx second, the clean summary last and quietest —
// rather than always stacking block/warn/clean together as one unit.

// Held back items stop an install outright; the tinted panel keeps them the
// loudest thing on the card no matter what else is going on.
export function BlockedFindings({ rows }: { rows: ItemSafety[] }) {
  if (rows.length === 0) return null;
  return (
    <div className="space-y-3 rounded-lg border border-critical/30 bg-critical/5 p-3">
      <SectionLabel className="text-critical">Held back</SectionLabel>
      {rows.map((row) => (
        <BlockedItem key={`${row.kind}:${row.name}:${row.harness}`} row={row} />
      ))}
    </div>
  );
}

export function SafetyCleanSummary({ rows }: { rows: ItemSafety[] }) {
  if (rows.length === 0) return null;
  const total = rows.length;
  const sentences = [
    `${total} item${total === 1 ? "" : "s"} checked — nothing found.`,
    ...groupSkipped(rows).map((group) => {
      const noun = group.kind
        ? kindLabel(group.kind, group.count).toLowerCase()
        : `item${group.count === 1 ? "" : "s"}`;
      return `${group.count} ${noun} ${skipReasonShort(group.reason)}.`;
    }),
  ];
  return <p className="text-sm text-muted-foreground">{sentences.join(" ")}</p>;
}
