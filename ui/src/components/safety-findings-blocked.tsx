import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { Finding, ItemSafety } from "@/bindings";
import { FindingLine } from "@/components/safety-findings";
import { StatusDot } from "@/components/status-dot";
import { Badge } from "@/components/ui/badge";
import { BLOCKED_SECTION_EXPLAINER, BLOCKED_SECTION_TITLE } from "@/lib/copy";
import { findingHeadline } from "@/lib/finding-headlines";
import { SEVERITY_RANK } from "@/lib/group-findings";
import {
  type BlockedGroup,
  groupBlocked,
  type RuleGroup,
} from "@/lib/group-findings-blocked";
import {
  hookDisplayName,
  kindLabel,
  moreItemsLabel,
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

// One held-back rule reuses the warn list's finding anatomy exactly — same
// severity lane, same order, same wording — so the two lists read as one
// system rather than two dialects of the same information.
function ruleGroupAsFinding(group: RuleGroup): Finding {
  return {
    rule: group.rule,
    severity: group.severity,
    location: group.locations[0] ?? "",
    message: group.message,
    remediation: group.remediation,
  };
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
        className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2.5 text-left hover:bg-critical/5"
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
        <div className="flex flex-col gap-3 border-t border-critical/20 px-3 py-3.5">
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
            <FindingLine
              key={`${ruleGroup.rule}:${ruleGroup.message}`}
              finding={ruleGroupAsFinding(ruleGroup)}
              locations={ruleGroup.locations}
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
    <div className="flex flex-col gap-2 rounded-lg border border-critical/30 bg-critical/5 p-3">
      <div className="flex flex-col gap-1">
        <h3 className="text-[13px] font-semibold text-critical">
          {BLOCKED_SECTION_TITLE}
        </h3>
        <p className="text-[13px] text-muted-foreground">
          {BLOCKED_SECTION_EXPLAINER}
        </p>
      </div>
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
