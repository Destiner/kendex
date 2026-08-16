import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { ItemSafety } from "@/bindings";
import { FindingLine } from "@/components/safety-findings";
import { StatusDot } from "@/components/status-dot";
import { Badge } from "@/components/ui/badge";
import { FEWER_ITEMS_LABEL } from "@/lib/copy";
import { findingHeadline } from "@/lib/finding-headlines";
import {
  type ConcernGroup,
  concernDetails,
  type FindingItem,
  groupByConcern,
  groupFindings,
} from "@/lib/group-findings";
import {
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
// headline already carries the finding, so the chip stays terse. Items of
// mixed kinds fall back to a bare count rather than picking a winner.
function scopeChipLabel(items: FindingItem[]): string {
  if (items.length === 1) {
    const only = items[0];
    return only.kind === "hook" ? hookDisplayName(only.name) : only.name;
  }
  const kinds = new Set(items.map((item) => item.kind));
  if (kinds.size > 1) return `${items.length} items`;
  return `${items.length} ${kindLabel(items[0].kind, items.length).toLowerCase()}`;
}

const COLLAPSE_THRESHOLD = 6;

// The names of what this concern touched — a plugin called
// `chrome@openai-bundled` is what a person recognises, where the directory
// it happens to live in is not. Past a handful the rest are a click away,
// because a real plugin set (20+) prints a wall nobody reads to the end.
function AffectedList({ concern }: { concern: ConcernGroup }) {
  const [expanded, setExpanded] = useState(false);
  const items = concern.items;
  const visible = expanded ? items : items.slice(0, COLLAPSE_THRESHOLD);
  const hiddenCount = items.length - visible.length;
  const canCollapse = items.length > COLLAPSE_THRESHOLD;
  return (
    <div className="flex flex-col gap-1.5 text-[13px]">
      <p className="font-medium text-foreground/70">
        Affects {items.length}{" "}
        {affectedLabel(items, concern.findings[0].location)}
      </p>
      <div className="flex flex-wrap gap-1">
        {visible.map((item) => (
          <Badge
            key={`${item.harness}:${item.kind}:${item.name}`}
            variant="outline"
            className="max-w-full truncate font-normal"
          >
            {item.kind === "hook" ? hookDisplayName(item.name) : item.name}
          </Badge>
        ))}
        {canCollapse ? (
          <button
            type="button"
            onClick={() => setExpanded((e) => !e)}
            className="inline-flex items-center gap-0.5 px-1 text-xs text-muted-foreground hover:text-foreground hover:underline"
          >
            {expanded ? FEWER_ITEMS_LABEL : moreItemsLabel(hiddenCount)}
          </button>
        ) : null}
      </div>
    </div>
  );
}

// One disclosure row per concern: collapsed to a single plain-English line
// so the stack reads like a checklist, not a wall of engine text. Opening
// it says what it means, what to do, and what it touched — in that order,
// once each.
function ConcernRow({ concern }: { concern: ConcernGroup }) {
  const [open, setOpen] = useState(false);
  const details = concernDetails(concern);

  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2.5 text-left hover:bg-muted/40"
      >
        <StatusDot tone={SEVERITY_DOT_TONE[concern.severity]} />
        <span className="min-w-0 flex-1 truncate text-sm font-medium">
          {findingHeadline(concern.rule, details[0].finding.message)}
        </span>
        <Badge variant="outline" className="shrink-0 font-normal">
          {scopeChipLabel(concern.items)}
        </Badge>
        {open ? (
          <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
        )}
      </button>
      {open ? (
        <div className="flex flex-col gap-4 border-t bg-muted/20 px-3 py-3.5">
          {details.map((detail) => (
            <FindingLine
              key={detail.finding.message}
              finding={detail.finding}
              locations={detail.locations}
            />
          ))}
          <AffectedList concern={concern} />
        </div>
      ) : null}
    </div>
  );
}

// The number of disclosure rows the safety list would render — used by the
// section header's same-line count, computed here rather than in
// group-findings.ts since it's a display concern, not grouping logic.
export function safetyGroupCount(rows: ItemSafety[]): number {
  return groupByConcern(groupFindings(rows)).length;
}

export function SafetyWarnings({ rows }: { rows: ItemSafety[] }) {
  const concerns = groupByConcern(groupFindings(rows));
  if (concerns.length === 0) return null;
  return (
    <div className="divide-y divide-border overflow-hidden rounded-lg border">
      {concerns.map((concern) => (
        <ConcernRow key={concern.rule} concern={concern} />
      ))}
    </div>
  );
}
