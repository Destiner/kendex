// One place the app counts what the audit found, so the sidebar badge, the
// status footer, Home and the Review cards can never quote different
// numbers for the same thing.
//
// The engine emits one drift row per harness an item targets, so a skill
// present for five tools is five rows and one thing. A person counts the
// thing. Merging happens inside each scope: the same name in two projects
// is genuinely two items, and folding those together would undercount.
import type { AuditView, DriftRow } from "@/bindings";
import { heldBackCount } from "@/lib/derive";
import { mergeDriftRows } from "@/lib/drift-merge";

export interface AuditCounts {
  /** Writes vstack is ready to make: install, update, remove. */
  changes: number;
  /** On disk, but vstack was never asked to look after it. Not a debt —
   *  adopting is an offer the user takes up, so it is counted apart from
   *  the work that is actually queued. */
  unmanaged: number;
  /** Installs the safety gate is holding back until someone rules on them. */
  blocked: number;
}

function countMerged(views: AuditView[], keep: (row: DriftRow) => boolean) {
  return views.reduce(
    (sum, view) => sum + mergeDriftRows(view.drift.filter(keep)).length,
    0,
  );
}

export function auditCounts(views: AuditView[]): AuditCounts {
  return {
    changes: countMerged(views, (row) => row.state !== "unmanaged"),
    unmanaged: countMerged(views, (row) => row.state === "unmanaged"),
    blocked: heldBackCount(views),
  };
}

/** What the Review page has waiting for a person: work to apply, plus
 *  judgments only they can make. */
export function needsReviewCount(counts: AuditCounts): number {
  return counts.changes + counts.blocked;
}
