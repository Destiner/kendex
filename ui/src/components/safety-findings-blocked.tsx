import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { ItemSafety } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { StatusDot } from "@/components/status-dot";
import { Badge } from "@/components/ui/badge";
import { findingHeadline } from "@/lib/finding-headlines";
import { SEVERITY_RANK } from "@/lib/group-findings";
import {
  type BlockedGroup,
  groupBlocked,
  type RuleGroup,
  relativeLocations,
} from "@/lib/group-findings-blocked";
import {
  FEWER_ITEMS_LABEL,
  hookDisplayName,
  kindLabel,
  moreItemsLabel,
  SEVERITY_BADGES,
  SEVERITY_LABELS,
  toolName,
} from "@/lib/labels";

// A row where every rule was skipped has not been audited, and showing
// nothing would read as an audit that passed. It gets a line of its own
// saying what could not be looked at. Prefixed with the harness name once a
// blocked entry spans more than one, so a note about Codex isn't read as
// also true of Pi.
function BlockedRowNotes({
  row,
  harnessPrefix,
}: {
  row: ItemSafety;
  harnessPrefix: string;
}) {
  return (
    <>
      {row.override.state === "stale" ? (
        <p className="text-xs text-muted-foreground">
          {harnessPrefix}You accepted this before, but {row.override.why}.
        </p>
      ) : null}
      {row.override.state === "active" ? (
        <p className="text-xs text-muted-foreground">
          {harnessPrefix}You read these findings and accepted them, so this
          stays installed.
        </p>
      ) : null}
      {row.skipped.length > 0 ? (
        <p className="text-xs text-muted-foreground">
          {harnessPrefix}Not fully checked here: {row.skipped.length} rule
          {row.skipped.length === 1 ? "" : "s"} had nothing to read —{" "}
          {row.skipped[0].reason}
        </p>
      ) : null}
    </>
  );
}

const LOCATION_COLLAPSE_THRESHOLD = 6;

// The locations a rule-group hit, with their shared directory printed once
// on its own quiet line instead of on every entry — a rule firing at four
// call sites in the same skill folder otherwise repeats that folder's path
// four times before the reader reaches the part that differs.
function LocationList({ locations }: { locations: string[] }) {
  const [expanded, setExpanded] = useState(false);
  const { prefix, relative } = relativeLocations(locations);
  const visible = expanded
    ? relative
    : relative.slice(0, LOCATION_COLLAPSE_THRESHOLD);
  const hiddenCount = relative.length - visible.length;
  const canCollapse = relative.length > LOCATION_COLLAPSE_THRESHOLD;
  return (
    <div className="space-y-0.5">
      {prefix ? (
        <p className="break-all font-mono text-muted-foreground/60">{prefix}</p>
      ) : null}
      <p className="break-words font-mono text-muted-foreground">
        {visible.join(", ")}
        {canCollapse ? (
          <button
            type="button"
            onClick={() => setExpanded((e) => !e)}
            className="ml-1 text-foreground hover:underline"
          >
            {expanded ? FEWER_ITEMS_LABEL : moreItemsLabel(hiddenCount)}
          </button>
        ) : null}
      </p>
    </div>
  );
}

// One rule's message and fix, printed once, with every location it hit
// listed beneath — the badge-column-then-content grid matches FindingLine
// in safety-findings.tsx so the expanded held-back row and the warn list
// read as one system.
function RuleGroupLine({ group }: { group: RuleGroup }) {
  return (
    <div className="flex items-start gap-2 text-xs">
      <Badge
        variant={SEVERITY_BADGES[group.severity]}
        className="mt-0.5 shrink-0"
      >
        {SEVERITY_LABELS[group.severity]}
      </Badge>
      <div className="min-w-0 flex-1 space-y-0.5">
        <p className="break-words text-muted-foreground">{group.message}</p>
        <p className="break-words text-muted-foreground/70">
          <span className="text-muted-foreground/50">↳ Fix: </span>
          {group.remediation}
        </p>
        <LocationList locations={group.locations} />
      </div>
    </div>
  );
}

function leadRuleGroup(groups: RuleGroup[]): RuleGroup {
  return groups.reduce((lead, group) =>
    SEVERITY_RANK[group.severity] > SEVERITY_RANK[lead.severity] ? group : lead,
  );
}

// One disclosure row per grouped held-back entry — same anatomy as
// FindingRow in safety-findings-affected.tsx: a dot, a headline, a scope
// chip, a chevron. Held back is always the loudest verdict there is, so the
// dot stays critical regardless of whether a row inside was later accepted;
// that nuance is said in prose once the row opens, not in the dot's color.
function BlockedGroupRow({ group }: { group: BlockedGroup }) {
  const [open, setOpen] = useState(false);
  const lead = leadRuleGroup(group.findingGroups);
  const extraCount = group.findingGroups.length - 1;
  const name = group.kind === "hook" ? hookDisplayName(group.name) : group.name;
  const harnesses = [...new Set(group.rows.map((row) => row.harness))];

  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex w-full cursor-pointer items-center gap-2 px-3 py-2.5 text-left"
      >
        <StatusDot tone="critical" />
        <span className="min-w-0 flex-1 truncate text-sm font-medium">
          {name} — {findingHeadline(lead.rule, lead.message)}
          {extraCount > 0 ? (
            <span className="font-normal text-muted-foreground">
              {" "}
              {moreItemsLabel(extraCount)}
            </span>
          ) : null}
        </span>
        <Badge variant="outline" className="shrink-0 font-normal">
          {kindLabel(group.kind)} · {harnesses.map(toolName).join(", ")}
        </Badge>
        {open ? (
          <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
        )}
      </button>
      {open ? (
        <div className="space-y-2 pb-3 pl-7 pr-3">
          {group.rows.map((row) => (
            <BlockedRowNotes
              key={row.harness}
              row={row}
              harnessPrefix={
                harnesses.length > 1 ? `${toolName(row.harness)}: ` : ""
              }
            />
          ))}
          {group.findingGroups.map((ruleGroup) => (
            <RuleGroupLine
              key={`${ruleGroup.rule}:${ruleGroup.message}`}
              group={ruleGroup}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

// Held back items stop an install outright; the tinted panel keeps them the
// loudest thing on the card no matter what else is going on. Rows sharing a
// skill's files across harnesses, or a rule repeating across locations, are
// merged before rendering — see groupBlocked in group-findings-blocked.ts.
export function BlockedFindings({ rows }: { rows: ItemSafety[] }) {
  const groups = groupBlocked(rows);
  if (groups.length === 0) return null;
  return (
    <div className="space-y-3 rounded-lg border border-critical/30 bg-critical/5 p-3">
      <SectionLabel className="text-critical">Held back</SectionLabel>
      <div className="divide-y divide-critical/20 rounded-md border border-critical/20 bg-background/40">
        {groups.map((group) => (
          <BlockedGroupRow
            key={group.rows
              .map((row) => `${row.kind}:${row.name}:${row.harness}`)
              .join("|")}
            group={group}
          />
        ))}
      </div>
    </div>
  );
}
