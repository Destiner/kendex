import { describe, expect, it } from "vitest";
import type { AuditView, DriftRow, HarnessId } from "@/bindings";
import { auditCounts, needsReviewCount } from "./audit-counts";

function drift(
  name: string,
  harness: HarnessId,
  state: DriftRow["state"],
  root?: string,
): DriftRow {
  return {
    kind: "skill",
    name,
    harness,
    scope: root ? { scope: "project", root } : { scope: "global" },
    state,
    detail: "",
  };
}

function view(rows: DriftRow[], root?: string): AuditView {
  return {
    scope: root ? { scope: "project", root } : { scope: "global" },
    drift: rows,
    plan: [],
    notes: [],
    warnings: [],
    safety: [],
    heldBack: [],
  };
}

describe("auditCounts", () => {
  it("counts one item installed for five tools once", () => {
    const tools: HarnessId[] = ["claude", "codex", "opencode", "cursor", "pi"];
    const rows = tools.map((h) => drift("agent-browser", h, "unmanaged"));

    expect(auditCounts([view(rows)])).toMatchObject({
      unmanaged: 1,
      changes: 0,
    });
  });

  it("keeps the same name in two projects apart", () => {
    const personal = view([drift("github", "claude", "unmanaged")]);
    const project = view([drift("github", "claude", "unmanaged", "/p")], "/p");

    expect(auditCounts([personal, project]).unmanaged).toBe(2);
  });

  it("separates queued work from what was never adopted", () => {
    const rows = [
      drift("a", "claude", "stale"),
      drift("b", "claude", "missing"),
      drift("c", "claude", "unmanaged"),
    ];

    expect(auditCounts([view(rows)])).toMatchObject({
      changes: 2,
      unmanaged: 1,
    });
  });

  it("leaves un-adopted items out of what needs reviewing", () => {
    const rows = [
      drift("a", "claude", "stale"),
      drift("c", "claude", "unmanaged"),
    ];

    expect(needsReviewCount(auditCounts([view(rows)]))).toBe(1);
  });
});
