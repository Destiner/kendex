import type { HarnessId } from "@/bindings";
import type { MergedDriftRow } from "@/lib/drift-merge";
import { toolName } from "@/lib/labels";
import { sameScope } from "@/stores/audit";
import { useScanStore } from "@/stores/scan";

export interface SharedLink {
  group: MergedDriftRow;
  /** The harness whose link to adopt through — the core resolves the
   *  target and takes every sibling link with it in one plan. */
  harness: HarnessId;
  /** The real folder the links resolve to. */
  target: string;
  /** Every tool whose install is a link at that folder. */
  tools: string[];
}

// An install that is a live symlink adopts the *target* — a folder the
// user may have pointed several tools at. That is a bigger move than
// adopting a plain folder (the old folder is trashed, and links vstack
// cannot see will break), so it gets a confirmation naming the folder and
// every tool reading it. Detection reads the scan, which resolves links.
export function sharedLinkOf(group: MergedDriftRow): SharedLink | null {
  const items = useScanStore.getState().result?.items ?? [];
  for (const row of group.installations) {
    const item = items.find(
      (it) =>
        it.kind === group.kind &&
        it.name === group.name &&
        it.harness === row.harness &&
        sameScope(it.scope, row.scope),
    );
    if (item?.fileState.state !== "symlink" || item.fileState.broken) {
      continue;
    }
    const target = item.fileState.target;
    const tools = items
      .filter(
        (it) =>
          it.kind === group.kind &&
          it.name === group.name &&
          sameScope(it.scope, row.scope) &&
          it.fileState.state === "symlink" &&
          !it.fileState.broken &&
          it.fileState.target === target,
      )
      .map((it) => toolName(it.harness));
    return {
      group,
      harness: row.harness,
      target,
      tools: [...new Set(tools)],
    };
  }
  return null;
}
