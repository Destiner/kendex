import type {
  CapabilityRow,
  Enforcement,
  HarnessId,
  ItemKind,
  KindCaps,
  OpSupport,
  ToggleDirection,
} from "@/bindings";

const PG: OpSupport = { project: true, global: true };
const P: OpSupport = { project: true, global: false };
const G: OpSupport = { project: false, global: true };
const NO: OpSupport = { project: false, global: false };

const cap = (
  observe: OpSupport,
  manage: OpSupport,
  toggle = manage,
  toggleDirection: ToggleDirection = "both",
  enforcement: Enforcement = "not-applicable",
): KindCaps => ({
  observe,
  adopt: manage,
  install: manage,
  toggle,
  remove: manage,
  refresh: manage,
  installsAs: null,
  toggleDirection,
  enforcement,
});

// A hook the tool runs, versus one it only reads as instructions.
const enforcedHook = (observe: OpSupport, manage: OpSupport): KindCaps =>
  cap(observe, manage, manage, "both", "enforced");
const advisoryHook = (observe: OpSupport, manage: OpSupport): KindCaps =>
  cap(observe, manage, manage, "both", "advisory");

// Codex retired its prompt directory: a command is written, toggled and
// removed as a skill, while the prompts it still loads are only read.
const codexCommand: KindCaps = {
  ...cap(G, PG),
  adopt: NO,
  installsAs: "skill",
};

// Mirrors crates/core/src/harness/caps.rs closely enough for UI gating;
// the real table stays the only authority inside the app itself.
const KIND_CAPS: Record<HarnessId, Partial<Record<ItemKind, KindCaps>>> = {
  claude: {
    agent: cap(PG, PG),
    skill: cap(PG, PG),
    hook: enforcedHook(PG, PG),
    command: cap(PG, PG),
    "mcp-server": cap(PG, PG),
    plugin: cap(G, NO, G),
  },
  codex: {
    agent: cap(PG, PG),
    skill: cap(PG, PG),
    hook: enforcedHook(PG, PG),
    command: codexCommand,
    "mcp-server": cap(PG, NO),
    plugin: cap(G, NO),
  },
  opencode: {
    agent: cap(PG, PG),
    skill: cap(PG, PG),
    hook: advisoryHook(PG, PG),
    command: cap(PG, NO),
    "mcp-server": cap(PG, NO),
    plugin: cap(PG, NO),
  },
  cursor: {
    agent: cap(P, P),
    skill: cap(P, P),
    hook: advisoryHook(P, P),
    command: cap(PG, NO),
    "mcp-server": cap(PG, NO),
  },
  pi: {
    agent: cap(PG, PG),
    skill: cap(PG, PG),
    command: cap(PG, NO),
    "pi-extension": cap(PG, PG),
  },
  // Read-only until their adapters land.
  gemini: {
    agent: cap(PG, NO),
    skill: cap(PG, NO),
    hook: enforcedHook(PG, NO),
    command: cap(PG, NO),
    "mcp-server": cap(PG, NO),
    plugin: cap(G, NO),
  },
  copilot: {
    agent: cap(PG, NO),
    // A repository file can add to Copilot's disabled lists but never
    // remove from them, so the switch only turns things off.
    skill: cap(PG, NO, NO, "disable-only"),
    "mcp-server": cap(PG, NO, NO, "disable-only"),
  },
};

const KINDS: ItemKind[] = [
  "agent",
  "skill",
  "hook",
  "command",
  "mcp-server",
  "plugin",
  "pi-extension",
];

export function capabilityTable(): CapabilityRow[] {
  const rows: CapabilityRow[] = [];
  for (const harness of Object.keys(KIND_CAPS) as HarnessId[]) {
    for (const kind of KINDS) {
      rows.push({
        harness,
        kind,
        caps: KIND_CAPS[harness][kind] ?? cap(NO, NO),
      });
    }
  }
  return rows;
}
