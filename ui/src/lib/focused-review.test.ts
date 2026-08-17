import { describe, expect, it } from "vitest";
import type { Finding, ItemSafety, Severity } from "@/bindings";
import { reviewQueue, stillOpen } from "./focused-review";

function row(name: string, severity: Severity, dismissed = false): ItemSafety {
  const finding: Finding = {
    rule: "r",
    severity,
    location: "SKILL.md:1",
    message: "m",
    remediation: "f",
  };
  return {
    kind: "skill",
    name,
    harness: "claude",
    scope: { scope: "global" },
    location: "",
    safety: { score: 90, deductions: [] },
    quality: null,
    findings: [finding],
    skipped: [],
    verdict: "warn",
    reasons: [],
    contentHash: "c",
    reviewHash: `hash-${name}`,
    provenance: null,
    override: { state: "absent" },
    decisions: [
      {
        fingerprint: "f",
        token: `skill:${name}:claude#f@hash-${name}`,
        state: dismissed
          ? {
              state: "dismissed",
              reason: "intended",
              dismissedAt: "2026-08-16T00:00:00Z",
            }
          : { state: "open", earlier: null },
      },
    ],
  };
}

describe("reviewQueue", () => {
  it("walks the most serious evidence first", () => {
    const queue = reviewQueue([
      row("low", "low"),
      row("high", "high"),
      row("mid", "medium"),
    ]);
    expect(queue.map((g) => g.items[0].name)).toEqual(["high", "mid", "low"]);
  });
});

describe("stillOpen", () => {
  it("skips a step something else already decided", () => {
    const [step] = reviewQueue([row("a", "high")]);
    expect(stillOpen(step, [row("a", "high")])).toBe(true);
    expect(stillOpen(step, [row("a", "high", true)])).toBe(false);
    expect(stillOpen(step, [])).toBe(false);
  });
});
