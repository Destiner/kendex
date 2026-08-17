// The ACME project's blocked items: the held-back rows the Review page's
// triage design is built against — one plain block, one skill blocked for
// two tools at once, and two config-entry kinds.
import type { Finding, HarnessId, ItemSafety } from "@/bindings";
import { accepted, decisionsFor } from "./fixture-decisions";
import { ACME, proj } from "./fixture-scopes";

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
  reviewHash: "a1b2c3d4e5f6",
  location: "",
  provenance: null,
  decisions: decisionsFor(
    "skill:scraper:claude",
    "a1b2c3d4e5f6",
    SCRAPER_FINDINGS,
  ),
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
  reviewHash: "visual-qa-shared",
  location: "",
  provenance: null,
  decisions: decisionsFor(
    `skill:visual-qa:${harness}`,
    "visual-qa-shared",
    VISUAL_QA_FINDINGS,
  ),
  override: { state: "absent" },
});

// You can accept a held-back item's findings and it stays installed, or
// accept them and have the content change since — both keep rendering
// inside the "Held back" panel, just with a note instead of a stop sign.
const LOG_UPLOADER_FINDINGS: Finding[] = [
  {
    rule: "credential-theft",
    severity: "critical",
    location: "SKILL.md:8",
    message: "reads an API token and includes it in an outbound request",
    remediation: "confirm the destination is one you trust before installing",
  },
];

const logUploaderSafety = (): ItemSafety => ({
  kind: "skill",
  name: "log-uploader",
  harness: "claude",
  scope: proj(ACME),
  safety: { score: 55, deductions: [] },
  quality: null,
  findings: LOG_UPLOADER_FINDINGS,
  skipped: [],
  verdict: "block",
  reasons: [
    "A serious finding holds an item back on its own, whatever the score.",
  ],
  contentHash: "log-uploader-v2",
  reviewHash: "log-uploader-v2",
  location: "",
  provenance: null,
  decisions: decisionsFor(
    "skill:log-uploader:claude",
    "log-uploader-v2",
    LOG_UPLOADER_FINDINGS,
  ),
  override: {
    state: "stale",
    why: "the skill's content has changed since you accepted it",
  },
});

const METRICS_RELAY_FINDINGS: Finding[] = [
  {
    rule: "broad-permissions",
    severity: "high",
    location: ".mcp.json:5",
    message: "requests filesystem access far beyond what it declares using",
    remediation: "narrow the requested scope, or drop it if it's unused",
  },
];

const metricsRelaySafety = (): ItemSafety => ({
  kind: "mcp-server",
  name: "metrics-relay",
  harness: "claude",
  scope: proj(ACME),
  safety: { score: 58, deductions: [] },
  quality: null,
  findings: METRICS_RELAY_FINDINGS,
  skipped: [],
  verdict: "block",
  reasons: [
    "A serious finding holds an item back on its own, whatever the score.",
  ],
  contentHash: "metrics-relay-v1",
  reviewHash: "metrics-relay-v1",
  location: "",
  provenance: null,
  decisions: decisionsFor(
    "mcp-server:metrics-relay:claude",
    "metrics-relay-v1",
    METRICS_RELAY_FINDINGS,
    [accepted("2026-08-10T09:12:00Z")],
  ),
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

// What the queued github update would install with: a finding the check
// noticed but does not hold back, so the apply preview can say it will be
// waiting once the update lands.
export function acmeQueued(): ItemSafety[] {
  const findings: Finding[] = [
    {
      rule: "dangerous-commands",
      severity: "medium",
      location: "SKILL.md:31",
      message:
        "`chmod 777` makes files writable by every account on the machine",
      remediation:
        "narrow the command to the exact path it needs, and let the user see it before it runs",
    },
  ];
  return [
    {
      kind: "skill",
      name: "github",
      harness: "claude",
      scope: proj(ACME),
      location: "",
      safety: { score: 92, deductions: [] },
      quality: null,
      findings,
      skipped: [],
      verdict: "warn",
      reasons: [],
      contentHash: "github-v3",
      reviewHash: "github-v3",
      provenance: "vanillagreencom/vstack",
      override: { state: "absent" },
      decisions: decisionsFor("skill:github:claude", "github-v3", findings),
    },
  ];
}
