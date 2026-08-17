// Builds the "Personal" (global) scope's drift rows and safety findings
// from the shared data in fixture-personal.ts, so the Review page has enough
// hooks and plugins to design triage at real scale. The ACME project's
// blocked items live next door in fixture-safety-acme.ts.
import type { DriftRow, ItemSafety } from "@/bindings";
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
import { GLOBAL } from "./fixture-scopes";

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
  reviewHash: `hook-${index}`,
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
  reviewHash: `clean-plugin-${index}`,
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
  reviewHash: `codex-plugin-${index}`,
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
