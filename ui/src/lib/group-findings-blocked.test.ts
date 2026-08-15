import { describe, expect, it } from "vitest";
import type { Finding, ItemSafety } from "@/bindings";
import {
  groupBlocked,
  groupFindingsByRule,
  relativeLocations,
} from "./group-findings-blocked";

const RULE_FINDING: Finding = {
  rule: "dangerous-commands",
  severity: "high",
  location: "/home/dana/skills/visual-qa/evals/grade.py:848",
  message: "runs a shell command built from unescaped input",
  remediation: "validate or escape the input before it reaches the shell",
};

function row(overrides: Partial<ItemSafety>): ItemSafety {
  return {
    kind: "skill",
    name: "visual-qa",
    harness: "codex",
    scope: { scope: "global" },
    safety: { score: 40, deductions: [] },
    quality: null,
    findings: [RULE_FINDING],
    skipped: [],
    verdict: "block",
    reasons: [],
    contentHash: "hash",
    override: { state: "absent" },
    ...overrides,
  };
}

describe("groupFindingsByRule", () => {
  it("collapses one rule firing at several locations into one entry", () => {
    const findings = [
      RULE_FINDING,
      {
        ...RULE_FINDING,
        location: "/home/dana/skills/visual-qa/process.py:89",
      },
      {
        ...RULE_FINDING,
        location: "/home/dana/skills/visual-qa/process.py:111",
      },
    ];
    const groups = groupFindingsByRule(findings);
    expect(groups).toHaveLength(1);
    expect(groups[0].locations).toEqual([
      RULE_FINDING.location,
      "/home/dana/skills/visual-qa/process.py:89",
      "/home/dana/skills/visual-qa/process.py:111",
    ]);
  });

  it("keeps rules with a different message or remediation apart", () => {
    const findings: Finding[] = [
      RULE_FINDING,
      { ...RULE_FINDING, message: "different message" },
      { ...RULE_FINDING, remediation: "different fix" },
    ];
    expect(groupFindingsByRule(findings)).toHaveLength(3);
  });

  it("keeps the highest severity across a rule's findings", () => {
    const findings: Finding[] = [
      { ...RULE_FINDING, severity: "medium" },
      { ...RULE_FINDING, severity: "critical" },
      { ...RULE_FINDING, severity: "low" },
    ];
    expect(groupFindingsByRule(findings)[0].severity).toBe("critical");
  });
});

describe("relativeLocations", () => {
  it("strips the longest shared directory across several locations", () => {
    const locations = [
      "/home/dana/skills/visual-qa/evals/grade.py:848",
      "/home/dana/skills/visual-qa/evals/grade.py:950",
      "/home/dana/skills/visual-qa/process.py:89",
      "/home/dana/skills/visual-qa/process.py:111",
    ];
    const { prefix, relative } = relativeLocations(locations);
    expect(prefix).toBe("/home/dana/skills/visual-qa/");
    expect(relative).toEqual([
      "evals/grade.py:848",
      "evals/grade.py:950",
      "process.py:89",
      "process.py:111",
    ]);
  });

  it("never cuts a shared prefix off mid-filename", () => {
    const locations = [
      "/home/dana/skills/x/SKILL.md:12",
      "/home/dana/skills/x/SKILL2.md:20",
    ];
    const { prefix, relative } = relativeLocations(locations);
    expect(prefix).toBe("/home/dana/skills/x/");
    expect(relative).toEqual(["SKILL.md:12", "SKILL2.md:20"]);
  });

  it("survives a single location without stripping anything useful to compare against", () => {
    const { prefix, relative } = relativeLocations([
      "/home/dana/skills/x/SKILL.md:12",
    ]);
    expect(prefix).toBe("");
    expect(relative).toEqual(["/home/dana/skills/x/SKILL.md:12"]);
  });

  it("survives an empty list", () => {
    expect(relativeLocations([])).toEqual({ prefix: "", relative: [] });
  });
});

describe("groupBlocked", () => {
  it("merges the same (kind, name) across harnesses when their finding sets are identical", () => {
    const codex = row({ harness: "codex" });
    const pi = row({ harness: "pi" });
    const groups = groupBlocked([codex, pi]);
    expect(groups).toHaveLength(1);
    expect(groups[0].rows.map((r) => r.harness)).toEqual(["codex", "pi"]);
  });

  it("does not merge the same (kind, name) across harnesses when their finding sets differ", () => {
    const codex = row({ harness: "codex" });
    const pi = row({
      harness: "pi",
      findings: [{ ...RULE_FINDING, location: "different-file.py:1" }],
    });
    const groups = groupBlocked([codex, pi]);
    expect(groups).toHaveLength(2);
  });

  it("groups the findings of a merged entry by rule", () => {
    const secondFinding: Finding = {
      rule: "rce",
      severity: "critical",
      location: "/home/dana/skills/visual-qa/evals/grade.py:12",
      message: "downloads a script from a URL and executes it directly",
      remediation:
        "pin and vendor the script instead of fetching it at runtime",
    };
    const codex = row({
      harness: "codex",
      findings: [RULE_FINDING, secondFinding],
    });
    const pi = row({ harness: "pi", findings: [RULE_FINDING, secondFinding] });
    const groups = groupBlocked([codex, pi]);
    expect(groups).toHaveLength(1);
    expect(groups[0].findingGroups.map((g) => g.rule)).toEqual([
      "dangerous-commands",
      "rce",
    ]);
  });

  it("keeps different names apart even with identical findings", () => {
    const a = row({ name: "visual-qa" });
    const b = row({ name: "other-skill" });
    expect(groupBlocked([a, b])).toHaveLength(2);
  });
});
