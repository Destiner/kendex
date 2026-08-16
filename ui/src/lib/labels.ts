// The product vocabulary, in one place: internal ids stay technical,
// everything a person reads goes through these maps.
import type {
  DriftRow,
  DriftState,
  HarnessId,
  ItemKind,
  Scope,
  Severity,
  Tag,
  Verdict,
} from "@/bindings";
import type { LibraryTab, Page, ToolsTab } from "@/stores/nav";

export const TOOL_NAMES: Record<HarnessId, string> = {
  claude: "Claude Code",
  codex: "Codex",
  opencode: "OpenCode",
  cursor: "Cursor",
  pi: "Pi",
  gemini: "Gemini CLI",
  copilot: "GitHub Copilot",
};

export const toolName = (id: HarnessId): string => TOOL_NAMES[id];

const KIND_LABELS: Record<ItemKind, { one: string; many: string }> = {
  agent: { one: "Agent", many: "Agents" },
  skill: { one: "Skill", many: "Skills" },
  hook: { one: "Hook", many: "Hooks" },
  command: { one: "Command", many: "Commands" },
  "mcp-server": { one: "MCP server", many: "MCP servers" },
  plugin: { one: "Plugin", many: "Plugins" },
  "pi-extension": { one: "Pi extension", many: "Pi extensions" },
};

export const kindLabel = (kind: ItemKind, count = 1): string =>
  count === 1 ? KIND_LABELS[kind].one : KIND_LABELS[kind].many;

// What each tag is called on screen. The written form is lower-case (it is
// what an author types into a file); the label is what a reader sees.
export const TAG_LABELS: Record<Tag, string> = {
  review: "Review",
  testing: "Testing",
  docs: "Docs",
  research: "Research",
  planning: "Planning",
  refactoring: "Refactoring",
  debugging: "Debugging",
  security: "Security",
  performance: "Performance",
  git: "Git",
  release: "Release",
  data: "Data",
  ui: "UI",
  integration: "Integration",
  automation: "Automation",
};
export const ALL_TAGS_FILTER_LABEL = "All tags";

// What a pending change would do, from the reader's side. "Orphaned" is the
// planner's word for an installed thing nothing declares any more; what a
// person needs to know is that nothing asks for it.
export const STATE_LABELS: Record<DriftState, string> = {
  missing: "will be installed",
  stale: "will be updated",
  orphaned: "nothing asks for it",
  unmanaged: "not managed yet",
  conflict: "needs a decision",
};

// How serious a safety finding is, said without security jargon.
export const SEVERITY_LABELS: Record<Severity, string> = {
  critical: "Serious",
  high: "Important",
  medium: "Worth a look",
  low: "Minor",
};

// What the safety check decided to do about an item.
export const VERDICT_LABELS: Record<Verdict, string> = {
  block: "Held back",
  warn: "Installs, with a warning",
  clean: "Nothing found",
};

// A collapsed row's dot: high and critical share the most urgent tone since
// both mean "worth stopping for"; low fades toward an already-checked item.
export const SEVERITY_DOT_TONE: Record<
  Severity,
  "critical" | "warning" | "muted"
> = {
  critical: "critical",
  high: "critical",
  medium: "warning",
  low: "muted",
};

export type BadgeVariant =
  | "default"
  | "secondary"
  | "destructive"
  | "outline"
  | "good"
  | "warning"
  | "critical"
  | "info";

export const STATE_BADGES: Record<DriftState, BadgeVariant> = {
  missing: "info",
  stale: "info",
  orphaned: "outline",
  unmanaged: "secondary",
  conflict: "warning",
};

// How serious a safety finding reads at a glance.
export const SEVERITY_BADGES: Record<Severity, BadgeVariant> = {
  critical: "critical",
  high: "warning",
  medium: "info",
  low: "secondary",
};

// What the safety check decided, at a glance.
export const VERDICT_BADGES: Record<Verdict, BadgeVariant> = {
  block: "critical",
  warn: "warning",
  clean: "good",
};

// "Personal" (Claude Code convention) lives in the home folder and applies everywhere; project items travel with the repo.
export function scopeName(scope: Scope): string {
  if (scope.scope === "global") return "Personal";
  return scope.root.split("/").pop() ?? scope.root;
}

export function scopePath(scope: Scope): string | null {
  return scope.scope === "global" ? null : scope.root;
}

// A hook's raw identifier is "<event>:<matcher>:<name>" or "<event>:<name>" — a person reads the trailing name; the full id stays in a mono line.
export function hookDisplayName(id: string): string {
  const parts = id.split(":");
  return parts[parts.length - 1] || id;
}

// Detail text that only restates the state pill next to it is dropped so
// the row reads once instead of twice.
const REDUNDANT_DRIFT_DETAILS: Partial<Record<DriftState, string>> = {
  missing: "not installed yet",
  stale: "newer content is available",
};

export function driftDetail(row: DriftRow): string | null {
  if (!row.detail) return null;
  return REDUNDANT_DRIFT_DETAILS[row.state] === row.detail ? null : row.detail;
}

// The engine's skip reason reads long repeated across many rows, so the
// clean-items summary uses this shortened paraphrase instead.
const SKIP_REASON_SHORT: Record<string, string> = {
  "the plugin's own files are not readable here — a declared plugin is one switch in a settings file until it is installed":
    "can't be fully checked until they're installed",
};

export function skipReasonShort(reason: string): string {
  return SKIP_REASON_SHORT[reason] ?? "can't be fully checked here yet";
}

// Affected-item disclosure copy — collapsed so a finding on 21 plugins isn't a wall of text.
export const moreItemsLabel = (hiddenCount: number): string =>
  `+${hiddenCount} more`;
export const PAGE_LABELS: Record<Page, string> = {
  home: "Home",
  review: "Review & apply",
  library: "Library",
  updates: "Updates",
  tools: "Tools & Projects",
  customize: "Customize",
  settings: "Settings",
  problems: "Problems",
  package: "Package",
};

const LIBRARY_TAB_LABELS: Record<LibraryTab, string> = {
  installed: "Installed",
  add: "Add from a catalog",
};

const TOOLS_TAB_LABELS: Record<ToolsTab, string> = {
  tools: "Tools",
  projects: "Projects",
};

// Where you are, in one line — pages without tabs read as just their name.
export function breadcrumbLabel(nav: {
  page: Page;
  libraryTab: LibraryTab;
  toolsTab: ToolsTab;
  /** The open package's display name, when the page is a package. */
  packageName?: string | null;
}): string {
  if (nav.page === "library") {
    return `${PAGE_LABELS.library} / ${LIBRARY_TAB_LABELS[nav.libraryTab]}`;
  }
  if (nav.page === "tools") {
    return `${PAGE_LABELS.tools} / ${TOOLS_TAB_LABELS[nav.toolsTab]}`;
  }
  if (nav.page === "package" && nav.packageName) {
    return `${PAGE_LABELS.library} / ${nav.packageName}`;
  }
  return PAGE_LABELS[nav.page];
}

/** How a package's name reads to a person — hooks carry display names. */
export function packageDisplayName(ref: {
  kind: ItemKind;
  name: string;
}): string {
  return ref.kind === "hook" ? hookDisplayName(ref.name) : ref.name;
}

// Settings page copy, kept here so the wording is reviewed in one place.
export const SETTINGS_SUBTITLE = "How vstack looks and behaves on this machine";
