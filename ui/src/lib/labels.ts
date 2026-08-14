// The product vocabulary, in one place: internal ids stay technical,
// everything a person reads goes through these maps.
import type {
  DriftRow,
  DriftState,
  HarnessId,
  ItemKind,
  Scope,
  Severity,
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

export const STATE_LABELS: Record<DriftState, string> = {
  missing: "not installed",
  stale: "out of date",
  orphaned: "left behind",
  unmanaged: "not managed yet",
  conflict: "needs attention",
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

// "Personal" follows the ecosystem convention (Claude Code skills docs):
// personal items live in the home folder and apply in every project;
// project items live in the repo and travel with it.
export function scopeName(scope: Scope): string {
  if (scope.scope === "global") return "Personal";
  return scope.root.split("/").pop() ?? scope.root;
}

export function scopePath(scope: Scope): string | null {
  return scope.scope === "global" ? null : scope.root;
}

// A hook's raw identifier is "<event>:<matcher>:<name>", or just
// "<event>:<name>" when there's no matcher — a person reads the trailing
// name; the full identifier stays available in a mono line beneath it.
export function hookDisplayName(id: string): string {
  const parts = id.split(":");
  return parts[parts.length - 1] || id;
}

// Some engine-written detail text only restates the state pill next to it
// ("out of date" badge, "newer content is available" detail say the same
// thing twice) — a known restatement is dropped so the row reads once.
const REDUNDANT_DRIFT_DETAILS: Partial<Record<DriftState, string>> = {
  missing: "not installed yet",
  stale: "newer content is available",
};

export function driftDetail(row: DriftRow): string | null {
  if (!row.detail) return null;
  return REDUNDANT_DRIFT_DETAILS[row.state] === row.detail ? null : row.detail;
}

// The engine writes a skip reason for one row; repeated across many rows it
// reads long, so the clean-items summary uses this shortened paraphrase.
const SKIP_REASON_SHORT: Record<string, string> = {
  "the plugin's own files are not readable here — a declared plugin is one switch in a settings file until it is installed":
    "can't be fully checked until they're installed",
};

export function skipReasonShort(reason: string): string {
  return SKIP_REASON_SHORT[reason] ?? "can't be fully checked here yet";
}

// Copy for the affected-item disclosure on a warning that names many
// items — collapsed by default so one finding on 21 plugins doesn't read
// as a wall of text.
export const moreItemsLabel = (hiddenCount: number): string =>
  `+${hiddenCount} more`;
export const FEWER_ITEMS_LABEL = "Show less";

export const PAGE_LABELS: Record<Page, string> = {
  home: "Home",
  review: "Review & apply",
  library: "Library",
  tools: "Tools & Projects",
  customize: "Customize",
  settings: "Settings",
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
}): string {
  if (nav.page === "library") {
    return `${PAGE_LABELS.library} / ${LIBRARY_TAB_LABELS[nav.libraryTab]}`;
  }
  if (nav.page === "tools") {
    return `${PAGE_LABELS.tools} / ${TOOLS_TAB_LABELS[nav.toolsTab]}`;
  }
  return PAGE_LABELS[nav.page];
}

export const BACK_LABEL = "Back";
export const WINDOW_CONTROL_LABELS = {
  minimize: "Minimize",
  maximize: "Maximize",
  close: "Close",
} as const;
