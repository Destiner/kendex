// Product prose: the sentences a person reads, as opposed to the vocabulary
// in labels.ts that names things. Kept apart so wording can be reviewed as
// writing, in one place, without wading through id-to-name maps.
//
// House style, applied throughout:
//   - Say what happened or what will happen, not what the code calls it.
//   - Name the thing the person is looking at, not the internal concept.
//   - Never claim a state the app has not checked.
export const FEWER_ITEMS_LABEL = "Show less";
export const morePlacesLabel = (count: number): string =>
  `and ${count} more place${count === 1 ? "" : "s"}`;
export const AFFECTS_LABEL = "Affects";

// "Safety" section label's same-line count and the clean-summary lead.
export const safetyGroupCountLabel = (count: number): string =>
  `${count} thing${count === 1 ? "" : "s"} worth a look`;
export const cleanSummaryLead = (total: number): string =>
  `${total} other thing${total === 1 ? "" : "s"} checked — nothing to report`;

export const SAFETY_HELP =
  "Strict catches more but sometimes flags things that are actually fine. Lenient trusts more, and only stops the riskiest items.";

// Review & apply page copy: what the page is, and what "managing" an item
// buys you — said once here so "Start managing" doesn't need to explain
// itself on every row.
export const REVIEW_SUBTITLE =
  "What vstack would change, and what it found while looking. Nothing is written until you apply.";
// This list scores what is on disk right now, not what a plan would write —
// so every row here is a thing the tools will load the next time they start.
// "Held back" describes what vstack refuses to do with it, and must never be
// read as "this isn't on your machine".
export const BLOCKED_SECTION_TITLE = "Serious problems found";
export const BLOCKED_SECTION_EXPLAINER =
  "These are on your machine now and your tools will load them. vstack won't install or update them until you've read the findings.";

// Says what you get, not what the app calls the state you'd be leaving.
export const UNMANAGED_SECTION_EXPLAINER =
  "Things already on your machine that vstack didn't put there. Hand one over and it gets kept up to date, safety-checked, and copied to every tool you use.";
export const START_MANAGING_LABEL = "Start managing";
// The apply flow, said as what will happen rather than as what the engine
// calls it. "Orphan" is a word for whoever wrote the planner; the person
// reading this wants to know something will be deleted and what it is.
export const APPLY_DIALOG_TITLE = "Apply these changes?";
export const APPLY_DIALOG_BODY =
  "vstack will update the files it manages. Nothing else on your machine is touched.";
export const APPLY_CONFIRM_LABEL = "Apply changes";
export const APPLY_BUTTON_LABEL = "Apply changes…";
export const NOTHING_TO_DO_HERE = "Nothing to do here";
// Every attention row leads to the same page, so they all say so the same
// way. Four different verbs for one destination read as four destinations.
export const REVIEW_ACTION_LABEL = "Open Review & apply";

// A project's one-line summary, so a closed panel still says what is inside
// it. Written as counted nouns rather than jargon: "2 changes ready" beats
// "2 drift rows", and a person can decide whether to open it from this line
// alone.
export function scopeSummaryLabel(counts: {
  changes: number;
  blocked: number;
  concerns: number;
  unmanaged: number;
}): string | null {
  const parts: string[] = [];
  if (counts.blocked > 0) {
    parts.push(
      counts.blocked === 1
        ? "1 serious problem"
        : `${counts.blocked} serious problems`,
    );
  }
  if (counts.changes > 0) {
    parts.push(
      counts.changes === 1
        ? "1 change ready"
        : `${counts.changes} changes ready`,
    );
  }
  if (counts.concerns > 0) {
    parts.push(
      counts.concerns === 1
        ? "1 thing worth a look"
        : `${counts.concerns} things worth a look`,
    );
  }
  if (counts.unmanaged > 0) {
    parts.push(`${counts.unmanaged} not managed yet`);
  }
  return parts.length > 0 ? parts.join(" · ") : null;
}
export const removeLeftBehindLabel = (count: number): string =>
  count === 1
    ? "Also delete 1 item nothing asks for any more"
    : `Also delete ${count} items nothing asks for any more`;
export const startManagingAllLabel = (count: number): string =>
  `Start managing all ${count}`;
export const showAllItemsLabel = (count: number): string => `Show all ${count}`;
export const HIDE_ITEMS_LABEL = "Hide";
export const adoptedToastLabel = (name: string): string =>
  `Now managing ${name}`;

// Home page copy: attention keeps its own subtitle since it's the lead;
// the other two sections are self-explanatory under their SectionLabel.
export const HOME_SUBTITLE = "What needs your attention, and what changed";
// "You're all caught up · Everything matches what you've chosen to install"
// left a person to work out what was compared with what. This says the thing
// that is actually true and what will happen next.
export const ALL_CAUGHT_UP_TITLE = "Nothing needs your attention";
export const ALL_CAUGHT_UP_DETAIL =
  "Every tool has the skills and agents you asked for. Anything new shows up here.";
// A file's timestamp can say that it changed and when — not who changed it
// or why — so the copy claims exactly that and no more.
export const RECENTLY_CHANGED_HELP =
  "Skills, agents and hooks whose files changed most recently.";
export const RECENT_ACTIVITY_EMPTY = "Nothing on this machine has changed yet.";

export const TAGS_ROW_LABEL = "For";

// Adding projects. Says what a project *is* to vstack — somewhere it will
// keep in sync — rather than naming the field twice over.
export const ADD_PROJECT_HELP =
  "Point vstack at a repository and it keeps that project's tools in sync too.";
export const SCAN_FOLDER_HELP =
  "Or look through a folder for repositories to add.";
export const NO_PROJECTS_FOUND = "Nothing that looks like a project in there.";

// "Add from a catalog". A catalog is a git repo of shareable skills and
// agents; a bundle is a named set inside one. Both are said in terms of what
// they get you rather than what they are.
export const BUNDLES_HELP =
  "Ready-made sets from your catalogs — install everything in one go.";
export const NO_BUNDLES_YET =
  "Nothing to show yet. Add a catalog and any sets it offers appear here.";
export const CATALOGS_HELP =
  "Git repositories of skills and agents you can install from.";
export const NO_CATALOGS_YET =
  "No catalogs yet. Add one and everything it offers becomes installable.";

// The one toggle on an item. It was a button reading "Turn off", which said
// what the click does but never what the state is or what turning it off
// costs you — a switch shows the state, and the sentence under it says the
// files stay put.
export const ENABLED_LABEL = "Enabled";
export const ENABLED_HELP =
  "Your tools load this. Switch it off and the files stay where they are — the tools just stop reading them.";

// Library flyout's open-actions menu.
export const OPEN_IN_LABEL = "Open in…";
export const OPEN_IN_FILE_BROWSER_LABEL = "File browser";
export const OPEN_IN_EDITOR_LABEL = "Editor";
export const EDITOR_ERROR_TITLE = "Couldn't open the editor";
export const EDITOR_ERROR_STEPS = [
  "Install VSCodium, VS Code, Cursor, Zed, or Sublime — or set VSTACK_EDITOR",
];
export const FILE_BROWSER_ERROR_TITLE = "Couldn't open the file browser";

export const BACK_LABEL = "Back";
export const WINDOW_CONTROL_LABELS = {
  minimize: "Minimize",
  maximize: "Maximize",
  close: "Close",
} as const;

// The status footer's left side: what the last scan is telling you.
export const SCANNING_LABEL = "Scanning…";
export const scanStatusLabel = (scannedAgo: string | null): string =>
  scannedAgo ? `Up to date · scanned ${scannedAgo}` : "Up to date";

// The status footer's right side: quiet counts that link to Review & apply.
export const pendingChangesLabel = (count: number): string =>
  count === 1 ? "1 change ready" : `${count} changes ready`;
export const heldBackFooterLabel = (count: number): string =>
  count === 1 ? "1 serious problem" : `${count} serious problems`;
