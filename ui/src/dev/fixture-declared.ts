import type {
  AuditView,
  ItemDecl_Serialize,
  Manifest_Serialize,
  SourceRow,
} from "@/bindings";
import { ACME, API, GLOBAL, proj } from "./fixture-scopes";

export function views(): AuditView[] {
  const acme = proj(ACME);
  return [
    { scope: GLOBAL, drift: [], plan: [], notes: [] },
    {
      scope: acme,
      drift: [
        {
          kind: "skill",
          name: "github",
          harness: "claude",
          scope: acme,
          state: "stale",
          detail: "the source has newer content than what is installed",
        },
        {
          kind: "hook",
          name: "guard",
          harness: "codex",
          scope: acme,
          state: "missing",
          detail: "declared for codex but not installed yet",
        },
        {
          kind: "skill",
          name: "scratch",
          harness: "claude",
          scope: acme,
          state: "unmanaged",
          detail: ".claude/skills/scratch exists but nothing manages it",
        },
        {
          kind: "agent",
          name: "old-helper",
          harness: "claude",
          scope: acme,
          state: "orphaned",
          detail: "recorded from an earlier setup; nothing declares it anymore",
        },
      ],
      plan: [
        "refresh skill github → claude (content changed)",
        "install hook guard → codex",
      ],
      notes: [],
    },
    { scope: proj(API), drift: [], plan: [], notes: [] },
  ];
}

const decl = (source: string): ItemDecl_Serialize => ({
  source,
  enabled: true,
});

export function manifests(): Record<string, Manifest_Serialize> {
  return {
    global: {
      schema: 1,
      sources: { vstack: { repo: "vanillagreencom/vstack", enabled: true } },
      install: { harnesses: ["claude"], method: "symlink" },
      skills: { "code-review": decl("vstack") },
      commands: { "ship-it": decl("vstack") },
    },
    [ACME]: {
      schema: 1,
      sources: {
        vstack: { repo: "vanillagreencom/vstack", enabled: true },
        team: { path: "../team-catalog", enabled: true },
      },
      install: { harnesses: ["claude", "codex", "pi"], method: "symlink" },
      agents: { orch: decl("vstack"), reviewer: decl("vstack") },
      skills: { github: decl("vstack"), deploy: decl("vstack") },
      hooks: { guard: decl("vstack") },
      "mcp-servers": { postgres: decl("vstack") },
      "agent-skills": { orch: ["github", "deploy"], reviewer: ["github"] },
      "agent-launch-instructions": { all: "Prefer small, reviewable changes." },
      "agent-frontmatter": {
        claude: { orch: { model: "opus", color: "blue" } },
      },
    },
    [API]: {
      schema: 1,
      sources: { vstack: { repo: "vanillagreencom/vstack", enabled: true } },
      install: { harnesses: ["claude"], method: "symlink" },
      agents: { orch: decl("vstack") },
      skills: { github: decl("vstack") },
    },
  };
}

export function sources(): SourceRow[] {
  const vstack = {
    name: "vstack",
    reference: "vanillagreencom/vstack",
    isRemote: true,
    enabled: true,
    head: "9f31c2a",
  };
  return [
    { scope: GLOBAL, ...vstack, declaredItems: ["code-review", "ship-it"] },
    {
      scope: proj(ACME),
      ...vstack,
      declaredItems: [
        "orch",
        "reviewer",
        "github",
        "deploy",
        "guard",
        "postgres",
      ],
    },
    {
      scope: proj(ACME),
      name: "team",
      reference: "../team-catalog",
      isRemote: false,
      enabled: true,
      head: null,
      declaredItems: [],
    },
    { scope: proj(API), ...vstack, declaredItems: ["orch", "github"] },
  ];
}
