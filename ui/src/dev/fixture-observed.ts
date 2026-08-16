import type {
  DetectedHarness,
  HarnessId,
  ItemKind,
  ObservedItem,
  Scope,
} from "@/bindings";
import {
  CLAUDE_HOOK_IDS,
  CLEAN_PLUGINS,
  CODEX_PLUGINS,
  HOOK_SETTINGS_PATH,
  UNMANAGED_SKILLS,
} from "./fixture-personal";
import { ACME, API, GLOBAL, proj } from "./fixture-scopes";

const FILE: ObservedItem["fileState"] = { state: "file" };
const ENTRY: ObservedItem["fileState"] = { state: "config-entry" };
const link = (target: string): ObservedItem["fileState"] => ({
  state: "symlink",
  target,
  broken: false,
});

function item(
  kind: ItemKind,
  name: string,
  harness: HarnessId,
  scope: Scope,
  path: string,
  over: Partial<ObservedItem> = {},
): ObservedItem {
  return {
    kind,
    name,
    harness,
    scope,
    path,
    fileState: { state: "dir" },
    enabled: true,
    origin: "vanillagreencom/vstack",
    description: null,
    tags: [],
    modifiedAt: null,
    ...over,
  };
}

const GH = "Work with GitHub: branches, pull requests, releases";
const ORCH = "Coordinates multi-step work across other agents";

// A spread of ages so the Library table's "Updated" column has something
// to show beyond "—" — real mtimes, just picked for variety here.
const NOW = Math.floor(Date.now() / 1000);
const MINUTES_AGO = (n: number) => NOW - n * 60;
const HOURS_AGO = (n: number) => NOW - n * 3600;
const DAYS_AGO = (n: number) => NOW - n * 86400;

export function harnesses(): DetectedHarness[] {
  return [
    { harness: "claude", root: "~/.claude", version: "2.1.34" },
    { harness: "codex", root: "~/.codex", version: "0.58.0" },
    { harness: "opencode", root: "~/.config/opencode", version: null },
    { harness: "pi", root: "~/.pi", version: "1.4.2" },
  ];
}

export function items(): ObservedItem[] {
  return [
    item(
      "skill",
      "github",
      "claude",
      proj(ACME),
      `${ACME}/.claude/skills/github`,
      {
        fileState: link(`${ACME}/.agents/skills/github`),
        description: GH,
        modifiedAt: MINUTES_AGO(12),
      },
    ),
    item(
      "skill",
      "github",
      "codex",
      proj(ACME),
      `${ACME}/.agents/skills/github`,
      { description: GH, modifiedAt: MINUTES_AGO(12) },
    ),
    item("skill", "github", "pi", proj(ACME), `${ACME}/.agents/skills/github`, {
      description: GH,
    }),
    item(
      "skill",
      "deploy",
      "claude",
      proj(ACME),
      `${ACME}/.claude/skills/deploy`,
      {
        fileState: link(`${ACME}/.agents/skills/deploy`),
        description: "Ship to staging and production safely",
        modifiedAt: HOURS_AGO(5),
      },
    ),
    item(
      "skill",
      "scratch",
      "claude",
      proj(ACME),
      `${ACME}/.claude/skills/scratch`,
      { origin: null, description: "Experimental notes" },
    ),
    item(
      "skill",
      "code-review",
      "claude",
      GLOBAL,
      "~/.claude/skills/code-review",
      {
        fileState: link("~/.local/share/vstack2/rendered/skills/code-review"),
        description: "A structured checklist for reviewing changes",
        modifiedAt: DAYS_AGO(9),
      },
    ),
    item(
      "agent",
      "orch",
      "claude",
      proj(ACME),
      `${ACME}/.claude/agents/orch.md`,
      { fileState: FILE, description: ORCH },
    ),
    item(
      "agent",
      "orch",
      "codex",
      proj(ACME),
      `${ACME}/.codex/agents/orch.toml`,
      { fileState: FILE, description: ORCH },
    ),
    item(
      "agent",
      "reviewer",
      "claude",
      proj(ACME),
      `${ACME}/.claude/agents/reviewer.md`,
      {
        fileState: FILE,
        description: "Reviews changes before they merge",
        modifiedAt: DAYS_AGO(2),
      },
    ),
    item("hook", "guard", "claude", proj(ACME), `${ACME}/.claude/hooks/guard`, {
      fileState: FILE,
      description: "Runs checks before every commit",
      modifiedAt: HOURS_AGO(30),
    }),
    item(
      "command",
      "ship-it",
      "claude",
      GLOBAL,
      "~/.claude/commands/ship-it.md",
      {
        fileState: FILE,
        description: "Draft a release pull request from the current branch",
      },
    ),
    item("mcp-server", "postgres", "claude", proj(ACME), `${ACME}/.mcp.json`, {
      fileState: ENTRY,
      description: "Query the app database from the assistant",
    }),
    item(
      "plugin",
      "linear@marketplace",
      "claude",
      GLOBAL,
      "~/.claude/plugins",
      {
        fileState: ENTRY,
        origin: null,
        description: "Linear issue tracking integration",
      },
    ),
    item(
      "pi-extension",
      "pi-hooks",
      "pi",
      GLOBAL,
      "~/.pi/agent/settings.json",
      {
        fileState: ENTRY,
        description: "Hook support for Pi",
      },
    ),
    item(
      "skill",
      "github",
      "claude",
      proj(API),
      `${API}/.claude/skills/github`,
      { fileState: link(`${API}/.agents/skills/github`), description: GH },
    ),
    item(
      "agent",
      "orch",
      "claude",
      proj(API),
      `${API}/.claude/agents/orch.md`,
      {
        fileState: FILE,
        description: ORCH,
      },
    ),
    ...CLAUDE_HOOK_IDS.map((name) =>
      item("hook", name, "claude", GLOBAL, HOOK_SETTINGS_PATH, {
        fileState: ENTRY,
        description: "Runs on a Claude Code event",
      }),
    ),
    ...CLEAN_PLUGINS.map((name) =>
      item("plugin", name, "claude", GLOBAL, "~/.claude/plugins", {
        fileState: ENTRY,
        origin: null,
        description: "Declared, not yet installed",
      }),
    ),
    ...CODEX_PLUGINS.map((name) =>
      item("plugin", name, "codex", GLOBAL, "~/.codex/plugins", {
        fileState: ENTRY,
        origin: null,
        description: "Bundled with Codex",
      }),
    ),
    ...UNMANAGED_SKILLS.map((skill) =>
      item("skill", skill.name, skill.harness, GLOBAL, skill.path, {
        fileState: FILE,
        origin: null,
        description: skill.description,
      }),
    ),
  ];
}
