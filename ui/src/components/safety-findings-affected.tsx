import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { ItemSafety } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { FindingLine, ItemTitle } from "@/components/safety-findings";
import {
  type AffectedSetGroup,
  type FindingItem,
  groupByAffectedSet,
  groupFindings,
} from "@/lib/group-findings";
import { FEWER_ITEMS_LABEL, kindLabel, moreItemsLabel } from "@/lib/labels";

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

// One block per distinct affected item-set: every finding that hit that
// exact set stacks above a single affected list, so two rules landing on
// the same 21 plugins (Codex's bundled registry) don't print the wall twice.
function AffectedGroup({ group }: { group: AffectedSetGroup }) {
  if (group.items.length === 1) {
    const only = group.items[0];
    return (
      <div className="space-y-0.5">
        <ItemTitle row={only} />
        {group.findings.map((finding) => (
          <FindingLine
            key={`${finding.rule}:${finding.location}:${finding.message}`}
            finding={finding}
          />
        ))}
      </div>
    );
  }
  return (
    <div className="space-y-1 text-xs">
      {group.findings.map((finding) => (
        <FindingLine
          key={`${finding.rule}:${finding.location}:${finding.message}`}
          finding={finding}
        />
      ))}
      <AffectedList group={group} />
    </div>
  );
}

export function SafetyWarnings({ rows }: { rows: ItemSafety[] }) {
  const affectedGroups = groupByAffectedSet(groupFindings(rows));
  if (affectedGroups.length === 0) return null;
  return (
    <div className="space-y-2">
      <SectionLabel>Safety</SectionLabel>
      {affectedGroups.map((group) => (
        <AffectedGroup
          key={group.items
            .map((i) => `${i.harness}:${i.kind}:${i.name}`)
            .join("|")}
          group={group}
        />
      ))}
    </div>
  );
}
