import { describe, expect, it } from "vitest";
import type { Finding, ItemSafety, ItemWarning } from "@/bindings";
import {
  groupByAffectedSet,
  groupFindings,
  groupSkipped,
  groupWarnings,
  partitionSafety,
} from "./group-findings";

const FINDING: Finding = {
  rule: "dangerous-commands",
  severity: "high",
  location: "settings.json:17",
  message: "`mkfs` formats a filesystem",
  remediation: "narrow the command to the exact path it needs",
};

function row(overrides: Partial<ItemSafety>): ItemSafety {
  return {
    kind: "hook",
    name: "a-hook",
    harness: "claude",
    scope: { scope: "global" },
    safety: { score: 85, deductions: [] },
    quality: null,
    findings: [],
    skipped: [],
    verdict: "warn",
    reasons: [],
    contentHash: "hash",
    override: { state: "absent" },
    ...overrides,
  };
}

describe("partitionSafety", () => {
  it("splits rows into held-back, warn, and clean buckets", () => {
    const blockedNoOverride = row({ verdict: "block", name: "held" });
    const blockedOverridden = row({
      verdict: "block",
      name: "accepted",
      override: { state: "active" },
    });
    const warnRow = row({ verdict: "warn", name: "warned" });
    const cleanRow = row({ verdict: "clean", name: "clean" });

    const groups = partitionSafety([
      warnRow,
      cleanRow,
      blockedOverridden,
      blockedNoOverride,
    ]);

    expect(groups.warn).toEqual([warnRow]);
    expect(groups.clean).toEqual([cleanRow]);
    expect(groups.blocked.map((r) => r.name)).toEqual(["held", "accepted"]);
  });
});

describe("groupFindings", () => {
  it("dedupes an identical finding across many rows into one group", () => {
    const rows = ["a", "b", "c"].map((name) =>
      row({ name, findings: [FINDING] }),
    );
    const groups = groupFindings(rows);
    expect(groups).toHaveLength(1);
    expect(groups[0].items.map((i) => i.name)).toEqual(["a", "b", "c"]);
    expect(groups[0].message).toBe(FINDING.message);
  });

  it("keeps findings separate when rule, location, or message differ", () => {
    const rows = [
      row({ name: "a", findings: [FINDING] }),
      row({ name: "b", findings: [{ ...FINDING, location: "other:1" }] }),
      row({ name: "c", findings: [{ ...FINDING, message: "different" }] }),
    ];
    expect(groupFindings(rows)).toHaveLength(3);
  });

  it("gives a finding affecting exactly one row a group of one", () => {
    const groups = groupFindings([row({ name: "solo", findings: [FINDING] })]);
    expect(groups).toHaveLength(1);
    expect(groups[0].items).toHaveLength(1);
    expect(groups[0].items[0].name).toBe("solo");
  });
});

describe("groupByAffectedSet", () => {
  it("merges two findings that hit the identical set of items into one group", () => {
    const rows = ["a", "b", "c"].map((name) => row({ name, findings: [] }));
    const groups = groupFindings([
      { ...rows[0], findings: [FINDING] },
      { ...rows[1], findings: [FINDING] },
      { ...rows[2], findings: [FINDING] },
    ]);
    // Simulate a second, distinct finding hitting the same three items.
    const secondFinding: Finding = { ...FINDING, rule: "no-manifest" };
    const groups2 = groupFindings([
      { ...rows[0], findings: [secondFinding] },
      { ...rows[1], findings: [secondFinding] },
      { ...rows[2], findings: [secondFinding] },
    ]);
    const merged = groupByAffectedSet([...groups, ...groups2]);
    expect(merged).toHaveLength(1);
    expect(merged[0].findings).toHaveLength(2);
    expect(merged[0].items.map((i) => i.name)).toEqual(["a", "b", "c"]);
  });

  it("keeps findings apart when their affected item-sets differ", () => {
    const abc = groupFindings(
      ["a", "b", "c"].map((name) => row({ name, findings: [FINDING] })),
    );
    const otherFinding: Finding = { ...FINDING, rule: "no-manifest" };
    const ab = groupFindings(
      ["a", "b"].map((name) => row({ name, findings: [otherFinding] })),
    );
    const merged = groupByAffectedSet([...abc, ...ab]);
    expect(merged).toHaveLength(2);
  });

  it("preserves the order affected-set groups were first seen in", () => {
    const first = groupFindings([row({ name: "solo1", findings: [FINDING] })]);
    const second = groupFindings([
      row({ name: "solo2", findings: [{ ...FINDING, rule: "other-rule" }] }),
    ]);
    const merged = groupByAffectedSet([...first, ...second]);
    expect(merged.map((g) => g.items[0].name)).toEqual(["solo1", "solo2"]);
  });
});

describe("groupSkipped", () => {
  it("counts rows sharing a skip reason and tracks a shared kind", () => {
    const reason = "the plugin's own files are not readable here";
    const rows = ["p1", "p2", "p3"].map((name) =>
      row({
        kind: "plugin",
        name,
        verdict: "clean",
        skipped: [{ rule: "some-rule", reason }],
      }),
    );
    const groups = groupSkipped(rows);
    expect(groups).toEqual([{ reason, count: 3, kind: "plugin" }]);
  });

  it("ignores rows with nothing skipped and nulls the kind when it varies", () => {
    const reason = "shared reason";
    const rows = [
      row({ kind: "plugin", verdict: "clean", skipped: [] }),
      row({
        kind: "plugin",
        verdict: "clean",
        skipped: [{ rule: "r", reason }],
      }),
      row({
        kind: "skill",
        verdict: "clean",
        skipped: [{ rule: "r", reason }],
      }),
    ];
    const groups = groupSkipped(rows);
    expect(groups).toEqual([{ reason, count: 2, kind: null }]);
  });
});

describe("groupWarnings", () => {
  it("dedupes identical message+remediation and lists affected items", () => {
    const warnings: ItemWarning[] = [
      {
        kind: "skill",
        name: "one",
        harness: "claude",
        message: "could not parse frontmatter",
        remediation: "check the YAML syntax",
      },
      {
        kind: "skill",
        name: "two",
        harness: "codex",
        message: "could not parse frontmatter",
        remediation: "check the YAML syntax",
      },
      {
        kind: "skill",
        name: "three",
        harness: "claude",
        message: "a different problem",
        remediation: null,
      },
    ];
    const groups = groupWarnings(warnings);
    expect(groups).toHaveLength(2);
    const shared = groups.find((g) => g.items.length === 2);
    expect(shared?.items.map((i) => i.name)).toEqual(["one", "two"]);
  });
});
