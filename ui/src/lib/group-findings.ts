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
} from "@/bindings";
import { heldBack } from "@/lib/derive";

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

export interface AffectedSetGroup {
  items: FindingItem[];
  findings: FindingGroup[];
}

// Two rules can both fire on the exact same items — Codex records every
// bundled plugin in one registry file, so "untracked repository" and "no
// manifest" both land on all of them. Rendered as two FindingGroups, that
// prints the same affected-item wall twice. This merges any FindingGroups
// whose affected item-set is identical, so the wall renders once with every
// finding that hit it stacked above.
export function groupByAffectedSet(groups: FindingGroup[]): AffectedSetGroup[] {
  const ordered: AffectedSetGroup[] = [];
  const bySetKey = new Map<string, AffectedSetGroup>();
  for (const group of groups) {
    const key = group.items
      .map((item) => `${item.kind}:${item.name}:${item.harness}`)
      .sort()
      .join("|");
    let affectedSetGroup = bySetKey.get(key);
    if (!affectedSetGroup) {
      affectedSetGroup = { items: group.items, findings: [] };
      bySetKey.set(key, affectedSetGroup);
      ordered.push(affectedSetGroup);
    }
    affectedSetGroup.findings.push(group);
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
