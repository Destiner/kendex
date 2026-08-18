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
    "You trust where this content came from. The same content from anywhere else asks again.",
};

/** The reasons in the order the dialog offers them: the common call first,
 *  the one that binds a source last, since it needs a known source. */
export const REASON_ORDER: DismissReason[] = [
  "wrong-call",
  "intended",
  "trusted-source",
];

export const reasonPhrase = (reason: DismissReason): string =>
  `Ignored — ${REASON_LABELS[reason].toLowerCase()}`;

// The dismiss dialog. Where the record lands decides who inherits it, so
// the body says which file — the same honesty the accept dialog has.
export const IGNORE_LABEL = "Ignore…";
export const IGNORE_TITLE = "Ignore this finding?";
export const ignoreBody = (projectScope: boolean): string =>
  projectScope
    ? "Stops asking until this content changes. Saved into this project's vstack.toml, so anyone using the repository inherits it — pick a reason that is true for them too."
    : "Stops asking until this content changes. Saved in your personal manifest on this machine.";
export const IGNORE_CONFIRM = "Ignore";
export const ignoreManyTitle = (count: number): string =>
  `Ignore ${count} findings?`;
export const ignoreManyBody =
  "These are the same content seen through several tools, so one decision covers all of them.";
export const UNDO_LABEL = "Undo";
export const ignoredToast = (count: number): string =>
  count === 1 ? "Finding ignored" : `${count} findings ignored`;
export const TAKEN_BACK_TOAST = "Back to needing a decision";
export const earlierDecisionNote = (why: string): string =>
  `You ignored this before, but ${why}.`;
// One concern, several different files behind it: each is its own call,
// and the set can be settled in one go.
export const separatePiecesLabel = (count: number): string =>
  `${count} separate items`;
export const ignoreAllLabel = (count: number): string => `Ignore all ${count}…`;
export const UNDECIDABLE_HERE = "Content can't be read on this machine";
export const NO_SOURCE_TO_TRUST =
  "vstack didn't install this from a catalog, so there's no source to trust.";

// The apply preview: a warning-only install is not held back, and its
// findings can only be decided once it is on disk — so the preview says
// what will be waiting.
export const queuedDecisionsLabel = (count: number): string =>
  count === 1
    ? "1 thing flagged in what this installs — you'll decide on it here once it lands."
    : `${count} things flagged in what this installs — you'll decide on them here once they land.`;

// The Settings list of every recorded decision — acceptances and
// dismissals — with the way out of each.
export const DECISIONS_SECTION_TITLE = "Recorded decisions";
export const RECORDED_DECISIONS_LINK = "See recorded decisions";
export const DECISIONS_SECTION_EXPLAINER =
  "Findings you accepted or dismissed. Each covers one version of one item — change the file and the finding comes back. A project's decisions live in its vstack.toml, so teammates inherit them.";
export const TAKE_BACK_LABEL = "Take back";
export const FORGET_LABEL = "Forget";
export const NO_LONGER_INSTALLED = "The item is no longer installed here.";
export const noLongerApplies = (why: string): string =>
  `No longer applies: ${why}.`;
export const decisionsErrorTitle = (scope: string): string =>
  `Couldn't read ${scope}'s decisions`;
export const acceptedPhrase = (count: number): string =>
  `Accepted ${count} finding${count === 1 ? "" : "s"}`;
