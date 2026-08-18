import { describe, expect, it } from "vitest";
import type { HookDelivery } from "@/bindings";
import { hookDeliverySummary } from "@/lib/copy-customize";

// The line under a hook is built from delivery rows the engine computed —
// these tests pin the composition, so no string literal in the UI can
// claim an enforcement the engine didn't decide.
describe("hookDeliverySummary", () => {
  const row = (
    harness: HookDelivery["harness"],
    mode: HookDelivery["mode"],
  ): HookDelivery => ({ harness, mode, note: null });

  it("says where a hook runs and where it is only guidance", () => {
    const line = hookDeliverySummary([
      row("claude", "runs"),
      row("codex", "runs"),
      row("cursor", "instructions"),
    ]);
    expect(line).toBe(
      "Runs in Claude Code and Codex · guidance only in Cursor — nothing enforces it there",
    );
  });

  it("counts Claude's per-agent block as running", () => {
    expect(hookDeliverySummary([row("claude", "runs-in-agent-file")])).toBe(
      "Runs in Claude Code",
    );
  });

  it("names the harnesses a hook cannot run in at all", () => {
    expect(hookDeliverySummary([row("cursor", "unavailable")])).toBe(
      "Can't run in Cursor",
    );
  });

  it("says nothing for an empty set", () => {
    expect(hookDeliverySummary([])).toBe("");
  });
});
