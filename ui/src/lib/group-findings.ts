// Pure grouping/dedupe logic for the safety wall: the engine emits one
// ItemSafety per installation, so a hook declared seven times or a plugin
// installed four times repeats the exact same finding — this collapses
// those repeats before anything renders.
import type {
  Finding,
  HarnessId,
  ItemKind,
  ItemSafety,
  ItemWarning,
  Severity,
} from "@/bindings";
import { heldBack } from "@/lib/derive";

// Shared so a collapsed row can lead with whichever finding or rule-group is
// most serious, without every caller re-deriving the same ranking.
export const SEVERITY_RANK: Record<Severity, number> = {
  low: 0,
  medium: 1,
  high: 2,
  critical: 3,
};

export interface SafetyGroups {
  /** verdict "block" — held back or overridden; always rendered per item. */
  blocked: ItemSafety[];
  /** verdict "warn" — findings get deduped across these before rendering. */
  warn: ItemSafety[];
  /** verdict "clean" — collapsed to a single summary line. */
  clean: ItemSafety[];
}

export function partitionSafety(rows: ItemSafety[]): SafetyGroups {
  const blocked: ItemSafety[] = [];
  const warn: ItemSafety[] = [];
  const clean: ItemSafety[] = [];
  for (const row of rows) {
    if (row.verdict === "block") blocked.push(row);
    else if (row.verdict === "warn") warn.push(row);
    else clean.push(row);
  }
  // Rows nothing can be done about yet lead; an already-accepted one follows.
  blocked.sort((a, b) => Number(heldBack(b)) - Number(heldBack(a)));
  return { blocked, warn, clean };
}

export interface FindingItem {
  kind: ItemKind;
  name: string;
  harness: HarnessId;
}

export interface FindingGroup extends Finding {
  items: FindingItem[];
}

/** Dedupes findings across rows by (rule, location, message). */
export function groupFindings(rows: ItemSafety[]): FindingGroup[] {
  const groups = new Map<string, FindingGroup>();
  for (const row of rows) {
    for (const finding of row.findings) {
      const key = `${finding.rule}::${finding.location}::${finding.message}`;
      let group = groups.get(key);
      if (!group) {
        group = { ...finding, items: [] };
        groups.set(key, group);
      }
      group.items.push({
        kind: row.kind,
        name: row.name,
        harness: row.harness,
      });
    }
  }
  return [...groups.values()];
}

export interface ConcernGroup {
  rule: string;
  /** The most serious severity any of this rule's findings carried. */
  severity: Severity;
  items: FindingItem[];
  findings: FindingGroup[];
}

// One rule firing in four places is one concern to a person, not four —
// "downloads and runs code from the internet" said once, with everything it
// touched behind it, beats the same sentence stacked four times. Concerns
// come back worst-first so the list reads in order of what to look at.
export function groupByConcern(groups: FindingGroup[]): ConcernGroup[] {
  const ordered: ConcernGroup[] = [];
  const byRule = new Map<string, ConcernGroup>();
  const seenItems = new Map<string, Set<string>>();
  for (const group of groups) {
    let concern = byRule.get(group.rule);
    if (!concern) {
      concern = {
        rule: group.rule,
        severity: group.severity,
        items: [],
        findings: [],
      };
      byRule.set(group.rule, concern);
      seenItems.set(group.rule, new Set());
      ordered.push(concern);
    }
    concern.findings.push(group);
    if (SEVERITY_RANK[group.severity] > SEVERITY_RANK[concern.severity]) {
      concern.severity = group.severity;
    }
    const seen = seenItems.get(group.rule);
    if (!seen) throw new Error(`no item set for concern ${group.rule}`);
    for (const item of group.items) {
      const key = `${item.kind}:${item.name}:${item.harness}`;
      if (seen.has(key)) continue;
      seen.add(key);
      concern.items.push(item);
    }
  }
  return ordered.sort(
    (a, b) => SEVERITY_RANK[b.severity] - SEVERITY_RANK[a.severity],
  );
}

/** Distinct message+fix pairs within a concern, each with every place it fired. */
export interface ConcernDetail {
  finding: Finding;
  locations: string[];
}

// The same rule usually emits the same sentence everywhere it fires, so the
// expansion shows that sentence once and lists the places under it.
export function concernDetails(concern: ConcernGroup): ConcernDetail[] {
  const ordered: ConcernDetail[] = [];
  const byMessage = new Map<string, ConcernDetail>();
  for (const finding of concern.findings) {
    const key = `${finding.message}::${finding.remediation}`;
    let detail = byMessage.get(key);
    if (!detail) {
      detail = { finding, locations: [] };
      byMessage.set(key, detail);
      ordered.push(detail);
    }
    if (!detail.locations.includes(finding.location)) {
      detail.locations.push(finding.location);
    }
  }
  return ordered;
}

export interface SkipGroup {
  reason: string;
  count: number;
  /** The shared kind, or null when the reason spans more than one kind. */
  kind: ItemKind | null;
}

// Every row here already passed with nothing found; a skipped rule only
// says a rule had no bytes to read, not that anything is wrong. The first
// skipped rule's reason stands in for the row, matching how a single row
// already summarizes "not fully checked" today.
export function groupSkipped(cleanRows: ItemSafety[]): SkipGroup[] {
  const groups = new Map<string, SkipGroup>();
  for (const row of cleanRows) {
    if (row.skipped.length === 0) continue;
    const reason = row.skipped[0].reason;
    const group = groups.get(reason);
    if (!group) groups.set(reason, { reason, count: 1, kind: row.kind });
    else {
      group.count += 1;
      if (group.kind !== row.kind) group.kind = null;
    }
  }
  return [...groups.values()];
}

export interface WarningGroup {
  message: string;
  remediation: string | null;
  items: { kind: ItemKind; name: string }[];
}

/** Dedupes render/parse warnings across items by (message, remediation). */
export function groupWarnings(warnings: ItemWarning[]): WarningGroup[] {
  const groups = new Map<string, WarningGroup>();
  for (const warning of warnings) {
    const key = `${warning.message}::${warning.remediation ?? ""}`;
    let group = groups.get(key);
    if (!group) {
      group = {
        message: warning.message,
        remediation: warning.remediation ?? null,
        items: [],
      };
      groups.set(key, group);
    }
    group.items.push({ kind: warning.kind, name: warning.name });
  }
  return [...groups.values()];
}
