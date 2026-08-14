import type { ItemSafety, Severity } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { heldBack } from "@/lib/derive";
import {
  type BadgeVariant,
  kindLabel,
  SEVERITY_LABELS,
  toolName,
  VERDICT_LABELS,
} from "@/lib/labels";

const SEVERITY_BADGES: Record<Severity, BadgeVariant> = {
  critical: "destructive",
  high: "destructive",
  medium: "secondary",
  low: "outline",
};

// Two scores, side by side and never combined: one says whether the
// content is dangerous, the other whether it is well made, and only the
// first one can hold an install back.
function Scores({ row }: { row: ItemSafety }) {
  return (
    <span className="text-muted-foreground">
      Safety {row.safety.score}/100
      {row.quality ? ` · Quality ${row.quality.score}/100` : ""}
    </span>
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

export function SafetyFindings({ rows }: { rows: ItemSafety[] }) {
  const worth = rows.filter(
    (row) =>
      row.verdict !== "clean" ||
      row.findings.length > 0 ||
      row.skipped.length > 0,
  );
  if (worth.length === 0) return null;

  return (
    <div className="space-y-3 border-t pt-3">
      {worth.map((row) => (
        <div
          key={`${row.kind}:${row.name}:${row.harness}`}
          className="space-y-1 text-sm"
        >
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={heldBack(row) ? "destructive" : "secondary"}>
              {row.verdict === "block" && !heldBack(row)
                ? "Accepted by you"
                : VERDICT_LABELS[row.verdict]}
            </Badge>
            <span className="font-semibold">{row.name}</span>
            <span className="text-muted-foreground">
              {kindLabel(row.kind)} · {toolName(row.harness)}
            </span>
            <Scores row={row} />
          </div>
          {row.override.state === "stale" ? (
            <p className="text-xs text-muted-foreground">
              You accepted this before, but {row.override.why}.
            </p>
          ) : null}
          {row.override.state === "active" ? (
            <p className="text-xs text-muted-foreground">
              You read these findings and accepted them, so this stays
              installed.
            </p>
          ) : null}
          <NotChecked row={row} />
          {row.findings.map((finding) => (
            <div
              key={`${finding.rule}:${finding.location}:${finding.message}`}
              className="flex flex-wrap items-start gap-2 text-xs"
            >
              <Badge variant={SEVERITY_BADGES[finding.severity]}>
                {SEVERITY_LABELS[finding.severity]}
              </Badge>
              <span className="min-w-0 flex-1 break-words text-muted-foreground">
                {finding.message}
                <span className="block break-all font-mono">
                  {finding.location}
                </span>
                <span className="block">Fix: {finding.remediation}</span>
              </span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
