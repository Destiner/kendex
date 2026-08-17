import { describe, expect, it } from "vitest";
import type { ItemSafety, ItemWarning } from "@/bindings";
import { groupSkipped, groupWarnings } from "./group-notes";

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
    reviewHash: "review-hash",
    location: "",
    provenance: null,
    decisions: [],
    override: { state: "absent" },
    ...overrides,
  };
}

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
