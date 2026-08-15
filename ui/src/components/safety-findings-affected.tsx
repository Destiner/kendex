import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { Finding, ItemSafety, Severity } from "@/bindings";
import { FindingLine } from "@/components/safety-findings";
import { StatusDot } from "@/components/status-dot";
import { Badge } from "@/components/ui/badge";
import { findingHeadline } from "@/lib/finding-headlines";
import {
  type AffectedSetGroup,
  type FindingItem,
  groupByAffectedSet,
  groupFindings,
} from "@/lib/group-findings";
import {
  FEWER_ITEMS_LABEL,
  hookDisplayName,
  kindLabel,
  moreItemsLabel,
  SEVERITY_DOT_TONE,
} from "@/lib/labels";

// A finding affecting the collection of hooks in one settings file reads as
// "N hooks in settings.json"; anything else just gets the plain kind name —
// a plugin's declaration doesn't live in one file a person would recognize.
function affectedLabel(items: FindingItem[], location: string): string {
  const kind = items[0].kind;
  const label = kindLabel(kind, items.length).toLowerCase();
  if (kind !== "hook") return label;
  const file = location.split("/").pop()?.split(":")[0];
  return file ? `${label} in ${file}` : label;
}

// The scope chip on a collapsed row: a single item is named directly since
// there's nothing to count, otherwise it's the count and kind — the row's
// headline already carries the finding, so the chip stays terse.
function scopeChipLabel(group: AffectedSetGroup): string {
  if (group.items.length === 1) {
    const only = group.items[0];
    return only.kind === "hook" ? hookDisplayName(only.name) : only.name;
  }
  return `${group.items.length} ${kindLabel(group.items[0].kind, group.items.length).toLowerCase()}`;
}

const SEVERITY_RANK: Record<Severity, number> = {
  low: 0,
  medium: 1,
  high: 2,
  critical: 3,
};

// A set-group can carry findings of mixed severity (e.g. a critical and a
// low finding on the same 21 plugins) — the collapsed row leads with
// whichever is most serious, since that's the one worth surfacing.
function leadFinding(findings: Finding[]): Finding {
  return findings.reduce((lead, f) =>
    SEVERITY_RANK[f.severity] > SEVERITY_RANK[lead.severity] ? f : lead,
  );
}

const COLLAPSE_THRESHOLD = 4;

// Collapsed by default: a finding affecting a real plugin set (20+) prints
// a wall of mono identifiers nobody reads end to end. The first handful
// establishes what's affected; the rest is a click away.
function AffectedList({ group }: { group: AffectedSetGroup }) {
  const [expanded, setExpanded] = useState(false);
  const items = group.items;
  const visible = expanded ? items : items.slice(0, COLLAPSE_THRESHOLD);
  const hiddenCount = items.length - visible.length;
  const canCollapse = items.length > COLLAPSE_THRESHOLD;
  return (
    <p className="text-muted-foreground">
      Affects {items.length} {affectedLabel(items, group.findings[0].location)}:{" "}
      <span className="inline-flex flex-wrap gap-x-1">
        {visible.map((item, i) => (
          <span
            key={`${item.harness}:${item.kind}:${item.name}`}
            className="inline-block break-all font-mono"
          >
            {item.name}
            {i < visible.length - 1 ? "," : ""}
          </span>
        ))}
      </span>
      {canCollapse ? (
        <button
          type="button"
          onClick={() => setExpanded((e) => !e)}
          className="ml-1 inline-flex items-center gap-0.5 align-middle text-foreground hover:underline"
        >
          {expanded ? (
            <ChevronDown className="size-3" />
          ) : (
            <ChevronRight className="size-3" />
          )}
          {expanded ? FEWER_ITEMS_LABEL : moreItemsLabel(hiddenCount)}
        </button>
      ) : null}
    </p>
  );
}

// One disclosure row per affected-set group: collapsed to a single
// plain-English line so the stack reads like a checklist, not a wall of
// engine text. Opening it reveals every finding that hit this set, in full.
function FindingRow({ group }: { group: AffectedSetGroup }) {
  const [open, setOpen] = useState(false);
  const lead = leadFinding(group.findings);
  const extraCount = group.findings.length - 1;

  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex w-full cursor-pointer items-center gap-2 px-3 py-2.5 text-left"
      >
        <StatusDot tone={SEVERITY_DOT_TONE[lead.severity]} />
        <span className="min-w-0 flex-1 truncate text-sm font-medium">
          {findingHeadline(lead.rule, lead.message)}
          {extraCount > 0 ? (
            <span className="font-normal text-muted-foreground">
              {" "}
              {moreItemsLabel(extraCount)}
            </span>
          ) : null}
        </span>
        <Badge variant="outline" className="shrink-0 font-normal">
          {scopeChipLabel(group)}
        </Badge>
        {open ? (
          <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
        )}
      </button>
      {open ? (
        <div className="space-y-2 pb-3 pl-7 pr-3">
          {group.findings.map((finding) => (
            <FindingLine
              key={`${finding.rule}:${finding.location}:${finding.message}`}
              finding={finding}
            />
          ))}
          <AffectedList group={group} />
        </div>
      ) : null}
    </div>
  );
}

// The number of disclosure rows the safety list would render — used by the
// section header's same-line count, computed here rather than in
// group-findings.ts since it's a display concern, not grouping logic.
export function safetyGroupCount(rows: ItemSafety[]): number {
  return groupByAffectedSet(groupFindings(rows)).length;
}

export function SafetyWarnings({ rows }: { rows: ItemSafety[] }) {
  const affectedGroups = groupByAffectedSet(groupFindings(rows));
  if (affectedGroups.length === 0) return null;
  return (
    <div className="divide-y divide-border rounded-lg border">
      {affectedGroups.map((group) => (
        <FindingRow
          key={group.items
            .map((i) => `${i.harness}:${i.kind}:${i.name}`)
            .join("|")}
          group={group}
        />
      ))}
    </div>
  );
}
