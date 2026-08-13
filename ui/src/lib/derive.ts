import type { ItemKind, ObservedItem, ScanResult, Scope } from "@/bindings";

export type ScopeSelection = "all" | "global" | { project: string };

export function scopeLabel(scope: Scope): string {
  return scope.scope === "global" ? "global" : scope.root;
}

export function scopeMatches(
  item: ObservedItem,
  selection: ScopeSelection,
): boolean {
  if (selection === "all") return true;
  if (selection === "global") return item.scope.scope === "global";
  return (
    item.scope.scope === "project" && item.scope.root === selection.project
  );
}

export interface ItemFilter {
  scope: ScopeSelection;
  kind?: ItemKind;
  harness?: string;
  search?: string;
}

export function filterItems(
  items: ObservedItem[],
  filter: ItemFilter,
): ObservedItem[] {
  const needle = filter.search?.trim().toLowerCase();
  return items.filter((item) => {
    if (!scopeMatches(item, filter.scope)) return false;
    if (filter.kind && item.kind !== filter.kind) return false;
    if (filter.harness && item.harness !== filter.harness) return false;
    if (needle) {
      const haystack = `${item.name} ${item.description ?? ""}`.toLowerCase();
      if (!haystack.includes(needle)) return false;
    }
    return true;
  });
}

/** One logical item (kind + name) with every installation observed for it. */
export interface ItemGroup {
  key: string;
  kind: ItemKind;
  name: string;
  description: string | null;
  installations: ObservedItem[];
  harnesses: string[];
  /** True when several harnesses read the same physical artifact. */
  shared: boolean;
}

export function groupItems(items: ObservedItem[]): ItemGroup[] {
  const groups = new Map<string, ItemGroup>();
  for (const item of items) {
    const key = `${item.kind}:${item.name}`;
    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        kind: item.kind,
        name: item.name,
        description: item.description,
        installations: [],
        harnesses: [],
        shared: false,
      };
      groups.set(key, group);
    }
    group.installations.push(item);
    group.description ??= item.description;
    if (!group.harnesses.includes(item.harness))
      group.harnesses.push(item.harness);
  }
  for (const group of groups.values()) {
    const byPath = new Map<string, Set<string>>();
    for (const install of group.installations) {
      const set = byPath.get(install.path) ?? new Set();
      set.add(install.harness);
      byPath.set(install.path, set);
    }
    group.shared = [...byPath.values()].some((harnesses) => harnesses.size > 1);
  }
  return [...groups.values()].sort((a, b) => a.key.localeCompare(b.key));
}

export function countByKind(items: ObservedItem[]): Map<ItemKind, number> {
  const counts = new Map<ItemKind, number>();
  for (const item of items) {
    counts.set(item.kind, (counts.get(item.kind) ?? 0) + 1);
  }
  return counts;
}

export function projectScopes(result: ScanResult): string[] {
  const roots = new Set<string>();
  for (const item of result.items) {
    if (item.scope.scope === "project") roots.add(item.scope.root);
  }
  return [...roots].sort();
}

/** What a bundle carries, short enough to sit under its name. */
export function bundleSummary(members: string[]): string {
  if (members.length === 0) return "Carries nothing yet";
  const shown = members.slice(0, 4).join(", ");
  const rest = members.length - 4;
  return rest > 0 ? `${shown}, and ${rest} more` : shown;
}
