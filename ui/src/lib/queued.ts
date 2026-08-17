// What an apply will leave waiting for a decision. A dismissal is about
// installed bytes and cannot be made on a plan, so the honest thing to do
// before the write is to say how many findings will need one after it.
import type { AuditView, ItemSafety } from "@/bindings";
import { evidenceGroups, openOccurrences } from "@/lib/reviewable";

const identity = (row: ItemSafety) =>
  `${row.kind}:${row.name}:${row.harness}:${row.reviewHash}`;

/** Open findings in what this apply would write, counted once per distinct
 *  evidence, leaving out content already installed unchanged — those
 *  findings are on the page already, and counting them twice would promise
 *  work the apply does not add. */
export function queuedDecisions(view: AuditView): number {
  const installed = new Set(view.safety.map(identity));
  const fresh = view.queued.filter((row) => !installed.has(identity(row)));
  return evidenceGroups(openOccurrences(fresh)).length;
}
