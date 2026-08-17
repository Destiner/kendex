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

// Review & apply page copy: what the page is, and what "managing" an item
// buys you — said once here so "Start managing" doesn't need to explain
// itself on every row.
export const REVIEW_SUBTITLE =
  "What vstack would change, and what it found while looking. Nothing is written until you apply.";
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
  open: number;
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
  if (counts.open > 0) {
    parts.push(
      counts.open === 1
        ? "1 finding needs your decision"
        : `${counts.open} findings need your decision`,
    );
  }
  if (counts.unmanaged > 0) {
    parts.push(`${counts.unmanaged} not managed yet`);
  }
  return parts.length > 0 ? parts.join(" · ") : null;
}
// The Review card's footnote about items vstack does not manage: they are
// not a debt, so they are counted here and acted on in the Library.
export const notManagedFootnote = (count: number): string =>
  count === 1
    ? "1 item on your machine isn't managed by vstack yet."
    : `${count} items on your machine aren't managed by vstack yet.`;
export const SEE_IN_LIBRARY_LABEL = "See them in the Library";
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
  "Look inside a folder for repositories, then add the ones you want.";
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
export const FORWARD_LABEL = "Forward";
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
export const decisionsFooterLabel = (count: number): string =>
  count === 1
    ? "1 thing needs your decision"
    : `${count} things need your decision`;

// Package page: files, versions, and the diff between them.
export const PACKAGE_FILES_TITLE = "Files";
export const PACKAGE_VERSION_TITLE = "Version";
export const README_TAG = "readme";
export const SHOWN_BY_DEFAULT_NOTE = "Shown when the package opens";
export const UPDATE_LABEL = "Update";
export const PREVIEW_CHANGES_LABEL = "Preview changes";
export const SWITCH_VERSION_LABEL = "Switch to this version";
export const COMPARE_WITH_INSTALLED_LABEL = "Compare with installed";
export const FOLLOW_SOURCE_LABEL = "Resume automatic updates";
export const INSTALLED_VERSION_TAG = "installed";
export const HELD_VERSION_TAG = "held here";
export const NO_VERSIONS_NOTE =
  "No version history yet — refresh the source to fetch it.";
export const BACK_TO_FILES_LABEL = "Back to files";
export const DIFF_TRUNCATED_NOTE =
  "This comparison is long; only the first part is shown.";
export const VERSION_ERROR_TITLE = "Couldn't switch versions";

// Updates page.
export const UPDATES_SUBTITLE =
  "Newer versions of what you have installed. Packages set to update automatically come current when you apply changes.";
export const UPDATES_EMPTY = "Everything is on its latest version.";
export const UPDATES_UNCHECKED_TITLE = "Couldn't be checked";
export const REMOVED_UPSTREAM_TAG = "No longer in its source";
export const UPDATE_ALL_LABEL = "Update all";
export const CHECK_FOR_UPDATES_LABEL = "Check for updates";
export const AUTO_UPDATE_LABEL = "Update automatically";
export const IGNORE_UPDATES_LABEL = "Stop notifying…";
export const ignoreConfirmTitle = (name: string): string =>
  `Stop notifying about ${name}?`;
export const IGNORE_CONFIRM_BODY =
  "It stays installed and can still be updated from its own page — it just leaves this list and the badge.";
export const IGNORE_CONFIRM_LABEL = "Stop notifying";
export const NOTIFY_AGAIN_LABEL = "Notify again";
export const hiddenUpdatesLabel = (count: number): string =>
  count === 1 ? "1 hidden update" : `${count} hidden updates`;
export const PINNED_UPDATE_TAG = "Held";
export const EDITED_UPDATE_TAG = "Edited by you";
export const UPDATE_ERROR_TITLE = "Couldn't update";
export const updatedToastLabel = (name: string): string => `Updated ${name}`;
export const UPDATED_ALL_TOAST = "Everything is up to date";

// Fork: what happens when the app finds files you edited by hand.
export const FORKED_BADGE_LABEL = "Forked";
export const FORK_NOTICE_TITLE = "You've changed this package's files";
export const FORK_NOTICE_DETAIL =
  "Updates are paused so your edits stay. Keep it as your own copy, see what changed, or discard the edits and go back to the catalog's version.";
export const KEEP_AS_FORK_LABEL = "Keep as my own";
export const VIEW_CHANGES_LABEL = "View changes";
export const DISCARD_EDITS_LABEL = "Discard edits…";
export const DISCARD_EDITS_CONFIRM_TITLE = "Discard your edits?";
export const DISCARD_EDITS_CONFIRM_BODY =
  "The catalog's version replaces your edits to this package, and your changes are gone. Keep them as your own copy instead if you're unsure.";
export const DISCARD_EDITS_CONFIRM_LABEL = "Discard edits";
export const FORK_ERROR_TITLE = "Couldn't keep the edits";
export const forkedToastLabel = (name: string): string =>
  `${name} is yours now — updates are paused`;
export const forkedAttentionTitle = (count: number): string =>
  count === 1
    ? "You've edited an installed package"
    : `You've edited ${count} installed packages`;
export const FORKED_ATTENTION_DETAIL =
  "Your changes are safe — nothing will overwrite them. Decide whether to keep each as your own copy.";

// The install-time ask, answered by default: installs keep themselves
// current unless the toast's one tap says otherwise.
export const installedAutoToastLabel = (name: string): string =>
  `Installed ${name} — it will keep itself up to date`;
export const UPDATE_MANUALLY_ACTION = "Update manually instead";
export const FOLLOW_SOURCE_TOAST = "Now updating automatically";
