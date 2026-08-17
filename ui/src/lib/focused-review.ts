// The queue a one-by-one review walks: every open piece of evidence in a
// scope, worst first, frozen when the walk starts. The page keeps moving
// underneath — each decision refreshes the scope — so the queue is a list
// of tokens taken once, and each step is checked against the live view
// before it is offered, never trusted from the moment it was taken.
import type { ItemSafety } from "@/bindings";
import { SEVERITY_RANK } from "@/lib/group-findings";
import {
  type EvidenceGroup,
  evidenceGroups,
  openOccurrences,
} from "@/lib/reviewable";

/** The evidence to walk, most serious first. Evidence with no token has
 *  nothing a walk could do with it, so it stays on the page and out of the
 *  queue. */
export function reviewQueue(rows: ItemSafety[]): EvidenceGroup[] {
  return evidenceGroups(openOccurrences(rows))
    .filter((group) => group.tokens.length > 0)
    .sort(
      (a, b) =>
        SEVERITY_RANK[b.finding.severity] - SEVERITY_RANK[a.finding.severity],
    );
}

/** Whether a queued step is still a live question on the current view: the
 *  same tokens are still open. A step decided from elsewhere — a toast
 *  Undo, another window — is skipped rather than shown as if it were new. */
export function stillOpen(step: EvidenceGroup, rows: ItemSafety[]): boolean {
  const live = new Set(
    evidenceGroups(openOccurrences(rows)).flatMap((group) => group.tokens),
  );
  return step.tokens.every((token) => live.has(token));
}
