import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { Finding, ItemSafety } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { heldBack } from "@/lib/derive";
import {
  type AffectedSetGroup,
  type FindingItem,
  groupByAffectedSet,
  groupFindings,
  groupSkipped,
  partitionSafety,
} from "@/lib/group-findings";
import {
  FEWER_ITEMS_LABEL,
  hookDisplayName,
  kindLabel,
  moreItemsLabel,
  SEVERITY_BADGES,
  SEVERITY_LABELS,
  skipReasonShort,
  toolName,
  VERDICT_BADGES,
  VERDICT_LABELS,
} from "@/lib/labels";

// A hook's raw identifier ("PreToolUse:*:claude-hook") is never the title —
// the trailing name is, with the full identifier kept in mono beneath it.
function ItemTitle({
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
      <span className="font-semibold">{name}</span>{" "}
      <span className="text-muted-foreground">
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

function FindingLine({ finding }: { finding: Finding }) {
  return (
    <div className="flex flex-wrap items-start gap-2 text-xs">
      <Badge variant={SEVERITY_BADGES[finding.severity]}>
        {SEVERITY_LABELS[finding.severity]}
      </Badge>
      <span className="min-w-0 flex-1 break-words text-muted-foreground">
        {finding.message}
        <span className="block break-all font-mono">{finding.location}</span>
        <span className="block">Fix: {finding.remediation}</span>
      </span>
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

function CleanSummary({ rows }: { rows: ItemSafety[] }) {
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

export function SafetyFindings({ rows }: { rows: ItemSafety[] }) {
  const { blocked, warn, clean } = partitionSafety(rows);
  if (blocked.length === 0 && warn.length === 0 && clean.length === 0) {
    return null;
  }
  const affectedGroups = groupByAffectedSet(groupFindings(warn));

  return (
    <div className="space-y-3 border-t pt-3">
      {blocked.map((row) => (
        <BlockedItem key={`${row.kind}:${row.name}:${row.harness}`} row={row} />
      ))}
      {affectedGroups.length > 0 ? (
        <div className="space-y-2">
          {affectedGroups.map((group) => (
            <AffectedGroup
              key={group.items
                .map((i) => `${i.harness}:${i.kind}:${i.name}`)
                .join("|")}
              group={group}
            />
          ))}
        </div>
      ) : null}
      <CleanSummary rows={clean} />
    </div>
  );
}
