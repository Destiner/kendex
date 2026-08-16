// Product prose for the safety surfaces: the held-back panel, accepting
// findings, the recorded-acceptances list, and taking over a shared
// folder. Split from copy.ts for the file line cap — same house style,
// same rules (see the top of copy.ts).

// "Safety" section label's same-line count and the clean-summary lead.
export const safetyGroupCountLabel = (count: number): string =>
  `${count} thing${count === 1 ? "" : "s"} worth a look`;
export const cleanSummaryLead = (total: number): string =>
  `${total} other thing${total === 1 ? "" : "s"} checked — nothing to report`;

export const SAFETY_HELP =
  "Strict catches more but sometimes flags things that are actually fine. Lenient trusts more, and only stops the riskiest items.";

// This list scores what is on disk right now, not what a plan would write —
// so every row here is a thing the tools will load the next time they start.
// "Held back" describes what vstack refuses to do with it, and must never be
// read as "this isn't on your machine".
export const BLOCKED_SECTION_TITLE = "Serious problems found";
export const BLOCKED_SECTION_EXPLAINER =
  "vstack won't install or update these until you've read the findings. Ones already on your machine still load in your tools.";
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

// The Settings list of recorded acceptances, and the way out of one.
export const ACCEPTED_SECTION_TITLE = "Accepted findings";
export const ACCEPTED_SECTION_EXPLAINER =
  "Serious findings someone read and accepted, so the item installs anyway. Withdrawing one holds the item back again — the next apply moves vstack's installed copy to the trash.";
export const WITHDRAW_LABEL = "Withdraw";
export const acceptedFindingsCountLabel = (count: number): string =>
  `${count} finding${count === 1 ? "" : "s"} accepted`;

// Taking over a folder that several tools read through links. The dialog
// names the real folder and every tool vstack knows is reading it; the
// last sentence is the one honest warning — links vstack cannot see will
// break, and there is no way to list them.
export const ADOPT_SHARED_TITLE = "Take over this shared folder?";
export const adoptSharedBody = (target: string, tools: string[]): string =>
  `${tools.join(" and ")} read this skill from ${target}. vstack moves the folder's content into its own keeping (the folder goes to the trash, recoverable) and gives each tool listed a link to vstack's copy, so they stay in sync. Anything else that points at the old folder will stop working.`;
export const ADOPT_SHARED_CONFIRM = "Take it over";
