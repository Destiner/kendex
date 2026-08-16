// Builds the "Personal" (global) scope's drift rows and safety findings
// from the shared data in fixture-personal.ts, plus the ACME project's
// blocked items, so the Review page has enough hooks, plugins, and held-back
// skills to design triage at real scale.
import type { DriftRow, Finding, HarnessId, ItemSafety } from "@/bindings";
import {
  CLAUDE_HOOK_IDS,
  CLEAN_PLUGINS,
  CLEAN_SKIP_REASON,
  CLEAN_SKIP_RULES,
  CODEX_FINDINGS,
  CODEX_PLUGINS,
  HOOK_FINDING,
  UNMANAGED_SKILLS,
} from "./fixture-personal";
import { ACME, GLOBAL, proj } from "./fixture-scopes";

const hookSafety = (name: string, index: number): ItemSafety => ({
  kind: "hook",
  name,
  harness: "claude",
  scope: GLOBAL,
  safety: { score: 85, deductions: [] },
  quality: null,
  findings: [HOOK_FINDING],
  skipped: [],
  verdict: "warn",
  reasons: [
    "A high-severity finding is worth a warning, though not enough on its own to hold this back.",
  ],
  contentHash: `hook-${index}`,
  override: { state: "absent" },
});

const cleanPluginSafety = (name: string, index: number): ItemSafety => ({
  kind: "plugin",
  name,
  harness: "claude",
  scope: GLOBAL,
  safety: { score: 100, deductions: [] },
  quality: null,
  findings: [],
  skipped: CLEAN_SKIP_RULES.map((rule) => ({
    rule,
    reason: CLEAN_SKIP_REASON,
  })),
  verdict: "clean",
  reasons: ["Nothing found, though its own files could not be read yet."],
  contentHash: `clean-plugin-${index}`,
  override: { state: "absent" },
});

const codexPluginSafety = (name: string, index: number): ItemSafety => ({
  kind: "plugin",
  name,
  harness: "codex",
  scope: GLOBAL,
  safety: { score: 92, deductions: [] },
  quality: null,
  findings: CODEX_FINDINGS,
  skipped: [],
  verdict: "warn",
  reasons: ["Nothing serious, but worth a look before you rely on it."],
  contentHash: `codex-plugin-${index}`,
  override: { state: "absent" },
});

export function personalSafety(): ItemSafety[] {
  return [
    ...CLAUDE_HOOK_IDS.map(hookSafety),
    ...CLEAN_PLUGINS.map(cleanPluginSafety),
    ...CODEX_PLUGINS.map(codexPluginSafety),
  ];
}

export function personalDrift(): DriftRow[] {
  return UNMANAGED_SKILLS.map((skill) => ({
    kind: "skill",
    name: skill.name,
    harness: skill.harness,
    scope: GLOBAL,
    state: "unmanaged",
    detail: skill.path,
  }));
}

// A skill installed for one tool at a time, one finding, blocked outright —
// the baseline case the held-back panel handles even without any grouping.
const SCRAPER_FINDINGS: Finding[] = [
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
];

const scraperSafety = (): ItemSafety => ({
  kind: "skill",
  name: "scraper",
  harness: "claude",
  scope: proj(ACME),
  safety: { score: 50, deductions: [] },
  quality: null,
  findings: SCRAPER_FINDINGS,
  skipped: [],
  verdict: "block",
  reasons: [
    "A serious finding holds an item back on its own, whatever the score.",
  ],
  contentHash: "a1b2c3d4e5f6",
  override: { state: "absent" },
});

// vstack keeps the same skill directory symlinked for every harness that
// declares it, so a skill installed for both Codex and Pi reads the exact
// same bytes and trips the exact same findings on both — the case
// groupBlocked exists to collapse into one entry instead of two verbatim
// panels. One rule fires at four call sites in the skill's own files, plus
// one distinct finding, to match how this actually shows up at real scale.
const VISUAL_QA_PATH = `${ACME}/.claude/skills/visual-qa`;
const VISUAL_QA_RULE_LOCATIONS = [
  `${VISUAL_QA_PATH}/evals/grade.py:848`,
  `${VISUAL_QA_PATH}/evals/grade.py:950`,
  `${VISUAL_QA_PATH}/process.py:89`,
  `${VISUAL_QA_PATH}/process.py:111`,
];
const VISUAL_QA_FINDINGS: Finding[] = [
  ...VISUAL_QA_RULE_LOCATIONS.map(
    (location): Finding => ({
      rule: "dangerous-commands",
      severity: "high",
      location,
      message: "runs a shell command built from unescaped input",
      remediation: "validate or escape the input before it reaches the shell",
    }),
  ),
  {
    rule: "rce",
    severity: "critical",
    location: `${VISUAL_QA_PATH}/evals/grade.py:12`,
    message: "downloads a script from a URL and executes it directly",
    remediation: "pin and vendor the script instead of fetching it at runtime",
  },
];

const visualQaSafety = (harness: HarnessId): ItemSafety => ({
  kind: "skill",
  name: "visual-qa",
  harness,
  scope: proj(ACME),
  safety: { score: 30, deductions: [] },
  quality: null,
  findings: VISUAL_QA_FINDINGS,
  skipped: [],
  verdict: "block",
  reasons: [
    "A serious finding holds an item back on its own, whatever the score.",
  ],
  contentHash: "visual-qa-shared",
  override: { state: "absent" },
});

// You can accept a held-back item's findings and it stays installed, or
// accept them and have the content change since — both keep rendering
// inside the "Held back" panel, just with a note instead of a stop sign.
const logUploaderSafety = (): ItemSafety => ({
  kind: "skill",
  name: "log-uploader",
  harness: "claude",
  scope: proj(ACME),
  safety: { score: 55, deductions: [] },
  quality: null,
  findings: [
    {
      rule: "credential-theft",
      severity: "critical",
      location: "SKILL.md:8",
      message: "reads an API token and includes it in an outbound request",
      remediation: "confirm the destination is one you trust before installing",
    },
  ],
  skipped: [],
  verdict: "block",
  reasons: [
    "A serious finding holds an item back on its own, whatever the score.",
  ],
  contentHash: "log-uploader-v2",
  override: {
    state: "stale",
    why: "the skill's content has changed since you accepted it",
  },
});

const metricsRelaySafety = (): ItemSafety => ({
  kind: "mcp-server",
  name: "metrics-relay",
  harness: "claude",
  scope: proj(ACME),
  safety: { score: 58, deductions: [] },
  quality: null,
  findings: [
    {
      rule: "broad-permissions",
      severity: "high",
      location: ".mcp.json:5",
      message: "requests filesystem access far beyond what it declares using",
      remediation: "narrow the requested scope, or drop it if it's unused",
    },
  ],
  skipped: [],
  verdict: "block",
  reasons: [
    "A serious finding holds an item back on its own, whatever the score.",
  ],
  contentHash: "metrics-relay-v1",
  override: { state: "active" },
});

export function acmeSafety(): ItemSafety[] {
  return [
    scraperSafety(),
    visualQaSafety("codex"),
    visualQaSafety("pi"),
    logUploaderSafety(),
    metricsRelaySafety(),
  ];
}

// The plan-time refusals: the same blocked items as the observed list (the
// next apply would rewrite them and the gate stops it again), so the demo
// shows the accept action on each blocked row.
export function acmeHeldBack(): ItemSafety[] {
  return [
    scraperSafety(),
    visualQaSafety("codex"),
    visualQaSafety("pi"),
    logUploaderSafety(),
  ];
}
