// Product prose for the safety surfaces: the decision zone, the held-back
// panel, accepting findings, and taking over a shared folder. Split from copy.ts for the file line cap — same house style,
// same rules (see the top of copy.ts).

// The zone only a person can clear: held-back installs first, then the
// findings nobody has ruled on. Its caption counts both halves.
export const DECISION_ZONE_TITLE = "Needs your decision";
export const cleanSummaryLead = (total: number): string =>
  `${total} item${total === 1 ? "" : "s"}, nothing to report`;
export const settledSummaryLead = (count: number): string =>
  `${count} finding${count === 1 ? "" : "s"} already decided`;

export const SAFETY_HELP =
  "Strict catches more, and flags more things that turn out fine. Lenient stops only the riskiest.";

// This list scores what is on disk right now, not what a plan would write —
// so every row here is a thing the tools will load the next time they start.
// "Held back" describes what vstack refuses to do with it, and must never be
// read as "this isn't on your machine".
export const BLOCKED_SECTION_EXPLAINER =
  "vstack won't install or update these. Open one, read what was found, then Accept to let it through. Copies already on your machine keep running.";
// The row for an install the gate stopped before it ever reached disk.
export const HELD_BACK_NOT_ON_DISK_NOTE =
  "Not installed — vstack stopped this one before it landed.";

// Accepting a held-back item. The action is reading the findings and
// choosing to install anyway; the record lands in a manifest, and *which*
// manifest decides who inherits the decision — so the dialog words the
// consequence per scope and claims nothing else.
export const ACCEPT_BLOCKED_LABEL = "Accept and install…";
// A held-back row the next apply would not write — an item already on the
// machine that vstack does not install. There is nothing to let through.
export const NOTHING_TO_ACCEPT =
  "Nothing to accept here — vstack isn't installing this one. It's already on your machine; remove it from the Library if you don't want it.";
export const ACCEPT_BLOCKED_TITLE = "Accept these findings?";
export const acceptBlockedBody = (projectScope: boolean): string =>
  projectScope
    ? "Saved into this project's vstack.toml, so anyone using the repository inherits it. It covers this version only — if the file changes, the block comes back."
    : "Saved in your personal manifest on this machine. It covers this version only — if the file changes, the block comes back.";
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
