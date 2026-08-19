// The cache vocabulary the marketplaces store and its readers share: the
// collision-free subscription key, and the invalidation every catalog-moving
// mutation runs.
import type { Scope } from "@/bindings";
import { useAuditStore } from "./audit";
import { resetPreinstallSafety } from "./preinstall-safety";
import { useScanStore } from "./scan";

/** One subscription's cache key: where it lives plus its alias, encoded so
 * a root or alias containing the delimiter can never collide with another
 * subscription's key. */
export const marketKey = (scope: Scope, source: string): string =>
  JSON.stringify([scope.scope === "global" ? null : scope.root, source]);

/** What lands after any mutation: the tables everywhere else stay current. */
export async function refreshDownstream() {
  await useScanStore.getState().refresh();
  await useAuditStore.getState().refresh();
}

export function without<T>(
  map: Record<string, T>,
  key: string,
): Record<string, T> {
  const { [key]: _, ...rest } = map;
  return rest;
}

/** A mutation that can change what any catalog offers empties every derived
 * cache — the pages re-read, and pre-install scores are re-asked, so nothing
 * keeps describing the commit before the change. */
export function dropCatalogCaches(set: (partial: object) => void) {
  set({ packages: {}, bundles: {}, about: {}, readErrors: {} });
  resetPreinstallSafety();
}
