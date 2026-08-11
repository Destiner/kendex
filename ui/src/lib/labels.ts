// The product vocabulary, in one place: internal ids stay technical,
// everything a person reads goes through these maps.
import type { DriftState, HarnessId, ItemKind, Scope } from "@/bindings";

export const TOOL_NAMES: Record<HarnessId, string> = {
  claude: "Claude Code",
  codex: "Codex",
  opencode: "OpenCode",
  cursor: "Cursor",
  pi: "Pi",
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

export type BadgeVariant = "default" | "secondary" | "destructive" | "outline";

export const STATE_BADGES: Record<DriftState, BadgeVariant> = {
  missing: "default",
  stale: "default",
  orphaned: "outline",
  unmanaged: "secondary",
  conflict: "destructive",
};

export function scopeName(scope: Scope): string {
  if (scope.scope === "global") return "Global";
  return scope.root.split("/").pop() ?? scope.root;
}

export function scopePath(scope: Scope): string | null {
  return scope.scope === "global" ? null : scope.root;
}
