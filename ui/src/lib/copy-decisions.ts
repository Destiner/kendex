// Product prose for decisions: the reasons a finding can be dismissed for,
// the dismiss dialog, and the recorded-decisions list. Same house style as
// copy.ts. Every reason is a claim about the content and reads the same to
// a teammate who inherits it from a project's vstack.toml — none of them
// is "I'm fine with the risk", because a project file is not the place for
// one person's tolerance.
import type { DismissReason } from "@/bindings";

export const REASON_LABELS: Record<DismissReason, string> = {
  "wrong-call": "Not actually a problem",
  intended: "Does this on purpose",
  "trusted-source": "From a source I trust",
};

export const REASON_HELP: Record<DismissReason, string> = {
  "wrong-call":
    "The check misread this — nothing here does what the finding says.",
  intended: "The flagged behaviour is what this item is for.",
  "trusted-source":
    "You trust where this content came from. Bound to that source — the same content from anywhere else asks again.",
};

/** The reasons in the order the dialog offers them: the common call first,
 *  the one that binds a source last, since it needs a known source. */
export const REASON_ORDER: DismissReason[] = [
  "wrong-call",
  "intended",
  "trusted-source",
];

export const reasonPhrase = (reason: DismissReason): string =>
  `Dismissed as ${REASON_LABELS[reason].toLowerCase()}`;

// The dismiss dialog. Where the record lands decides who inherits it, so
// the body says which file — the same honesty the accept dialog has.
export const DISMISS_LABEL = "Dismiss…";
export const DISMISS_TITLE = "Dismiss this finding?";
export const dismissBody = (projectScope: boolean): string =>
  projectScope
    ? "It stops asking, and stays dismissed until this content changes. The decision is saved into this project's vstack.toml, so anyone using the repository inherits it — pick the reason that is true for them too."
    : "It stops asking, and stays dismissed until this content changes. The decision is saved in your personal manifest on this machine.";
export const DISMISS_CONFIRM = "Dismiss";
export const dismissManyTitle = (count: number): string =>
  `Dismiss ${count} findings?`;
export const dismissManyBody =
  "These are the same content seen through several tools, so one decision covers all of them.";
export const UNDO_LABEL = "Undo";
export const dismissedToast = (count: number): string =>
  count === 1 ? "Finding dismissed" : `${count} findings dismissed`;
export const TAKEN_BACK_TOAST = "Dismissal taken back — the finding is back";
export const NO_SOURCE_TO_TRUST =
  "Nothing on this machine says where this content came from, so there is no source to trust.";

// Reviewing one finding at a time. Each step is one piece of evidence: an
// item, a finding, three reasons, and Skip. Twenty different plugins are
// twenty steps, because that is how many things there are to look at.
export const FOCUSED_REVIEW_LABEL = "Review one by one";
export const focusedProgress = (at: number, total: number): string =>
  `${at} of ${total}`;
export const focusedBody = (projectScope: boolean): string =>
  projectScope
    ? "Pick why this isn't a problem, or skip it. Decisions are saved into this project's vstack.toml, shared with everyone using it."
    : "Pick why this isn't a problem, or skip it. Decisions are saved in your personal manifest on this machine.";
export const FOCUSED_SKIP = "Skip";
export const FOCUSED_ALL_DONE = "That's everything";
export const FOCUSED_ALL_DONE_BODY =
  "Every finding here has been looked at. Anything you skipped is still waiting on the page.";

// The apply preview: a warning-only install is not held back, so before
// this its findings only appeared once it had landed. Now the preview says
// what will be waiting.
export const queuedDecisionsLabel = (count: number): string =>
  count === 1
    ? "The safety check found 1 thing in what this installs. It will be waiting for your decision here once it lands."
    : `The safety check found ${count} things in what this installs. They will be waiting for your decision here once they land.`;

// The Settings list of every recorded decision — acceptances and
// dismissals — with the way out of each.
export const DECISIONS_SECTION_TITLE = "Recorded decisions";
export const DECISIONS_SECTION_EXPLAINER =
  "Findings you accepted or dismissed. Each covers exactly the content it was made for — if that changes, it stops applying and the finding comes back. A project's decisions live in its vstack.toml, so anyone using the repository inherits them.";
export const TAKE_BACK_LABEL = "Take back";
export const FORGET_LABEL = "Forget";
export const NO_LONGER_INSTALLED = "The item is no longer installed here.";
export const noLongerApplies = (why: string): string =>
  `No longer applies: ${why}.`;
export const decisionsErrorTitle = (scope: string): string =>
  `Couldn't read ${scope}'s decisions`;
export const acceptedPhrase = (count: number): string =>
  `Accepted ${count} finding${count === 1 ? "" : "s"}`;
