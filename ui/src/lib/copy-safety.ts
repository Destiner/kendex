// Product prose for the safety surfaces: the decision zone, the held-back
// panel, accepting findings, and taking over a shared folder. Split from copy.ts for the file line cap — same house style,
// same rules (see the top of copy.ts).

// The zone only a person can clear: held-back installs first, then the
// findings nobody has ruled on. Its caption counts both halves.
export const DECISION_ZONE_TITLE = "Needs your decision";
export function decisionZoneLabel(blocked: number, open: number): string {
  const parts: string[] = [];
  if (blocked > 0) {
    parts.push(
      blocked === 1
        ? "1 install is held back until you rule on it"
        : `${blocked} installs are held back until you rule on them`,
    );
  }
  if (open > 0) {
    parts.push(
      open === 1
        ? "1 finding on installed content is waiting for a call"
        : `${open} findings on installed content are waiting for a call`,
    );
  }
  return parts.join(" · ");
}
export const cleanSummaryLead = (total: number): string =>
  `${total} other thing${total === 1 ? "" : "s"} checked — nothing to report`;
export const settledSummaryLead = (count: number): string =>
  count === 1
    ? "1 finding already decided"
    : `${count} findings already decided`;

export const SAFETY_HELP =
  "Strict catches more but sometimes flags things that are actually fine. Lenient trusts more, and only stops the riskiest items.";

// This list scores what is on disk right now, not what a plan would write —
// so every row here is a thing the tools will load the next time they start.
// "Held back" describes what vstack refuses to do with it, and must never be
// read as "this isn't on your machine".
export const BLOCKED_SECTION_EXPLAINER =
  "Serious problems. vstack won't install or update these until you've read the findings and accepted them; ones already on your machine still load in your tools.";
// The row for an install the gate stopped before it ever reached disk.
export const HELD_BACK_NOT_ON_DISK_NOTE =
  "Not on your machine — vstack was asked to install this and held it back.";

// Accepting a held-back item. The action is reading the findings and
// choosing to install anyway; the record lands in a manifest, and *which*
// manifest decides who inherits the decision — so the dialog words the
// consequence per scope and claims nothing else.
export const ACCEPT_BLOCKED_LABEL = "Accept and install…";
export const ACCEPT_BLOCKED_TITLE = "Accept these findings?";
export const acceptBlockedBody = (projectScope: boolean): string =>
  projectScope
    ? "Your acceptance is saved into this project's vstack.toml, so anyone who uses this repository inherits it. It covers exactly this version of the content — if the file changes, the block comes back."
    : "Your acceptance is saved in your personal manifest on this machine. It covers exactly this version of the content — if the file changes, the block comes back.";
export const ACCEPT_BLOCKED_CONFIRM = "Accept and install";

// Withdrawing an acceptance, from the recorded-decisions list.
export const WITHDRAW_LABEL = "Withdraw";

// Taking over a folder that several tools read through links. The dialog
// names the real folder and every tool vstack knows is reading it; the
// last sentence is the one honest warning — links vstack cannot see will
// break, and there is no way to list them.
export const ADOPT_SHARED_TITLE = "Take over this shared folder?";
export const adoptSharedBody = (target: string, tools: string[]): string =>
  `${tools.join(" and ")} read this skill from ${target}. vstack moves the folder's content into its own keeping (the folder goes to the trash, recoverable) and gives each tool listed a link to vstack's copy, so they stay in sync. Anything else that points at the old folder will stop working.`;
export const ADOPT_SHARED_CONFIRM = "Take it over";
