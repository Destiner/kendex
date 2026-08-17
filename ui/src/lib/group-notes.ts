// Grouping for the two kinds of note that ride beside findings: rules that
// had nothing to read on a clean row, and render or parse warnings. Kept
// apart from the finding grouping, which is what a decision targets.
import type { ItemKind, ItemSafety, ItemWarning } from "@/bindings";

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
