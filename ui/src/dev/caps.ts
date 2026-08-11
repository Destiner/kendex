import type {
  CapabilityRow,
  HarnessId,
  ItemKind,
  KindCaps,
  OpSupport,
} from "@/bindings";

const PG: OpSupport = { project: true, global: true };
const P: OpSupport = { project: true, global: false };
const G: OpSupport = { project: false, global: true };
const NO: OpSupport = { project: false, global: false };

const cap = (
  observe: OpSupport,
  manage: OpSupport,
  toggle = manage,
): KindCaps => ({
  observe,
  adopt: manage,
  install: manage,
  toggle,
  remove: manage,
  refresh: manage,
});

// Mirrors crates/core/src/harness/caps.rs closely enough for UI gating;
// the real table stays the only authority inside the app itself.
const KIND_CAPS: Record<HarnessId, Partial<Record<ItemKind, KindCaps>>> = {
  claude: {
    agent: cap(PG, PG),
    skill: cap(PG, PG),
    hook: cap(PG, PG),
    command: cap(PG, PG),
    "mcp-server": cap(PG, PG),
    plugin: cap(G, NO, G),
  },
  codex: {
    agent: cap(PG, PG),
    skill: cap(PG, PG),
    hook: cap(PG, PG),
    command: cap(G, NO),
    "mcp-server": cap(PG, NO),
    plugin: cap(G, NO),
  },
  opencode: {
    agent: cap(PG, PG),
    skill: cap(PG, PG),
    hook: cap(PG, PG),
    command: cap(PG, NO),
    "mcp-server": cap(PG, NO),
    plugin: cap(PG, NO),
  },
  cursor: {
    agent: cap(P, P),
    skill: cap(P, P),
    hook: cap(P, P),
    command: cap(PG, NO),
    "mcp-server": cap(PG, NO),
  },
  pi: {
    agent: cap(PG, PG),
    skill: cap(PG, PG),
    command: cap(PG, NO),
    "pi-extension": cap(PG, PG),
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
