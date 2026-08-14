import type {
  AuditView,
  BundleRow,
  ItemDecl_Serialize,
  Manifest_Serialize,
  SourceRow,
} from "@/bindings";
import { ACME, API, GLOBAL, proj } from "./fixture-scopes";

export function views(): AuditView[] {
  const acme = proj(ACME);
  return [
    { scope: GLOBAL, drift: [], plan: [], notes: [], warnings: [], safety: [] },
    {
      scope: acme,
      drift: [
        {
          kind: "skill",
          name: "github",
          harness: "claude",
          scope: acme,
          state: "stale",
          detail: "newer content is available",
        },
        {
          kind: "hook",
          name: "guard",
          harness: "codex",
          scope: acme,
          state: "missing",
          detail: "not installed yet",
        },
        {
          kind: "skill",
          name: "scratch",
          harness: "claude",
          scope: acme,
          state: "unmanaged",
          detail: `${ACME}/.claude/skills/scratch`,
        },
        {
          kind: "agent",
          name: "old-helper",
          harness: "claude",
          scope: acme,
          state: "orphaned",
          detail: "left over from an earlier setup; nothing needs it anymore",
        },
      ],
      plan: [
        "Update skill github for Claude Code",
        "Install hook guard for Codex",
        "Update the install record",
      ],
      notes: [],
      warnings: [],
      safety: [
        {
          kind: "skill",
          name: "scraper",
          harness: "claude",
          scope: acme,
          safety: {
            score: 50,
            deductions: [
              {
                rule: "credential-theft",
                location: "SKILL.md:12",
                severity: "critical",
                points: 25,
                repeat: false,
              },
              {
                rule: "dangerous-commands",
                location: "SKILL.md:20",
                severity: "high",
                points: 15,
                repeat: false,
              },
            ],
          },
          quality: null,
          findings: [
            {
              rule: "credential-theft",
              severity: "critical",
              location: "SKILL.md:12",
              message: "reads a credential file and sends it to a remote host",
              remediation:
                "remove the line that uploads the file, or install this skill only if you trust its source",
            },
            {
              rule: "dangerous-commands",
              severity: "high",
              location: "SKILL.md:20",
              message: "runs a shell command that deletes files without asking",
              remediation: "scope the command to a specific path, or drop it",
            },
          ],
          skipped: [],
          verdict: "block",
          reasons: [
            "A serious finding holds an item back on its own, whatever the score.",
          ],
          contentHash: "a1b2c3d4e5f6",
          override: { state: "absent" },
        },
      ],
    },
    {
      scope: proj(API),
      drift: [],
      plan: [],
      notes: [],
      warnings: [],
      safety: [],
    },
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

export function bundles(): BundleRow[] {
  const starter = {
    source: "vstack",
    name: "starter",
    description: "Everything a new repo needs",
    version: null,
    category: null,
    members: ["agent orch", "skill github", "skill deploy", "command ship-it"],
  };
  const review = {
    source: "vstack",
    name: "review",
    description: "Code review, end to end",
    version: "1.2.0",
    category: "quality",
    members: ["agent reviewer", "skill code-review"],
  };
  return [
    { scope: GLOBAL, ...starter, installed: false },
    { scope: GLOBAL, ...review, installed: true },
    { scope: proj(ACME), ...starter, installed: true },
    { scope: proj(ACME), ...review, installed: false },
  ];
}
