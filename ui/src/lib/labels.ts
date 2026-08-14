// The product vocabulary, in one place: internal ids stay technical,
// everything a person reads goes through these maps.
import type {
  DriftState,
  HarnessId,
  ItemKind,
  Scope,
  Severity,
  Verdict,
} from "@/bindings";

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
