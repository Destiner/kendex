import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { DismissReason, HarnessId, ItemSafety } from "@/bindings";
import { EvidenceList, IgnoreButton } from "@/components/finding-decide";
import { KindToolChips } from "@/components/kind-tool-chips";
import { FindingLine } from "@/components/safety-findings";
import { StatusDot } from "@/components/status-dot";
import { StatusLine } from "@/components/status-note";
import { Badge } from "@/components/ui/badge";
import { FEWER_ITEMS_LABEL } from "@/lib/copy";
import { earlierDecisionNote } from "@/lib/copy-decisions";
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
import { evidenceGroups, openOccurrences } from "@/lib/reviewable";

/** What a concern row can do: rule on the evidence behind it. */
interface Decide {
  projectScope: boolean;
  busy: boolean;
  onDismiss: (tokens: string[], reason: DismissReason) => void;
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

// What this concern touched, named the way a person recognises it: one row
// per piece of content, with the tools that load it. The same skill
// installed for four tools used to print the same name four times, which
// reads as four different problems.
function uniqueItems(items: FindingItem[]) {
  const byKey = new Map<
    string,
    { kind: FindingItem["kind"]; name: string; harnesses: HarnessId[] }
  >();
  for (const item of items) {
    const key = `${item.kind}:${item.name}`;
    const seen = byKey.get(key);
    if (seen) {
      if (!seen.harnesses.includes(item.harness))
        seen.harnesses.push(item.harness);
      continue;
    }
    byKey.set(key, {
      kind: item.kind,
      name: item.name,
      harnesses: [item.harness],
    });
  }
  return [...byKey.values()];
}

function AffectedList({ concern }: { concern: ConcernGroup }) {
  const [expanded, setExpanded] = useState(false);
  const items = uniqueItems(concern.items);
  const visible = expanded ? items : items.slice(0, COLLAPSE_THRESHOLD);
  const hiddenCount = items.length - visible.length;
  return (
    <div className="flex flex-col gap-1.5 text-[13px]">
      <p className="font-medium text-foreground">
        Affects {items.length}{" "}
        {affectedLabel(concern.items, concern.findings[0].location)}
      </p>
      <div className="flex flex-col gap-1.5">
        {visible.map((item) => (
          <div
            key={`${item.kind}:${item.name}`}
            className="flex flex-wrap items-center gap-2"
          >
            <span className="truncate">
              {item.kind === "hook" ? hookDisplayName(item.name) : item.name}
            </span>
            <KindToolChips kind={item.kind} harnesses={item.harnesses} />
          </div>
        ))}
        {items.length > COLLAPSE_THRESHOLD ? (
          <button
            type="button"
            onClick={() => setExpanded((e) => !e)}
            className="self-start text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
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
// once each — and then offers the decision. A concern behind which there
// is exactly one piece of evidence carries its Dismiss on the row itself;
// one that spans different content lists each piece with its own button,
// because one click there would be a rule-level mute across the fleet.
function ConcernRow({
  concern,
  decide,
}: {
  concern: ConcernGroup;
  decide: Decide;
}) {
  const [open, setOpen] = useState(false);
  const details = concernDetails(concern);
  const evidence = evidenceGroups(
    concern.findings.flatMap((group) => group.occurrences),
  );
  const single = evidence.length === 1 ? evidence[0] : null;

  return (
    <div>
      <div className="flex w-full items-center gap-2.5 px-3 py-2.5 hover:bg-muted/40">
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          aria-expanded={open}
          className="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5 text-left"
        >
          <StatusDot tone={SEVERITY_DOT_TONE[concern.severity]} />
          <span className="min-w-0 flex-1 truncate text-sm font-medium">
            {findingHeadline(concern.rule, details[0].finding.message)}
          </span>
          <Badge variant="outline" className="shrink-0 font-normal">
            {scopeChipLabel(concern.items)}
          </Badge>
        </button>
        {single ? (
          <IgnoreButton
            group={single}
            projectScope={decide.projectScope}
            busy={decide.busy}
            onDismiss={decide.onDismiss}
          />
        ) : null}
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          aria-expanded={open}
          aria-label={open ? "Collapse" : "Expand"}
          className="cursor-pointer"
        >
          {open ? (
            <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
          ) : (
            <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
          )}
        </button>
      </div>
      {open ? (
        <div className="flex flex-col gap-4 border-t bg-muted/20 px-3 py-3.5">
          {details.map((detail) => (
            <FindingLine
              key={detail.finding.message}
              finding={detail.finding}
              locations={detail.locations}
            />
          ))}
          {single?.earlier ? (
            <StatusLine tone="info">
              {earlierDecisionNote(single.earlier)}
            </StatusLine>
          ) : null}
          {single ? (
            <AffectedList concern={concern} />
          ) : (
            <EvidenceList
              groups={evidence}
              finding={details[0].finding.message}
              projectScope={decide.projectScope}
              busy={decide.busy}
              onDismiss={decide.onDismiss}
            />
          )}
        </div>
      ) : null}
    </div>
  );
}

// The number of disclosure rows the safety list would render — used by the
// section header's same-line count, computed here rather than in
// group-findings.ts since it's a display concern, not grouping logic.
export function safetyGroupCount(rows: ItemSafety[]): number {
  return groupByConcern(groupFindings(openOccurrences(rows))).length;
}

/** The open findings on installed content, one row per concern, each with
 *  the decision that settles it. */
export function SafetyWarnings({
  rows,
  ...decide
}: { rows: ItemSafety[] } & Decide) {
  const concerns = groupByConcern(groupFindings(openOccurrences(rows)));
  if (concerns.length === 0) return null;
  return (
    <div className="divide-y divide-border overflow-hidden rounded-lg border">
      {concerns.map((concern) => (
        <ConcernRow key={concern.rule} concern={concern} decide={decide} />
      ))}
    </div>
  );
}
