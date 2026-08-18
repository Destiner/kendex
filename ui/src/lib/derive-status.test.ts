import { describe, expect, it } from "vitest";
import type { ObservedItem } from "@/bindings";
import { groupItems, groupStatus } from "./derive";

function item(overrides: Partial<ObservedItem>): ObservedItem {
  return {
    kind: "skill",
    name: "deploy",
    harness: "claude",
    scope: { scope: "global" },
    path: "/h/.claude/skills/deploy",
    fileState: { state: "dir" },
    enabled: true,
    origin: null,
    description: null,
    tags: [],
    modifiedAt: null,
    vendor: null,
    ...overrides,
  };
}

const status = (items: ObservedItem[]) => groupStatus(groupItems(items)[0]);

describe("groupStatus", () => {
  it("is active while every copy is switched on and readable", () => {
    expect(status([item({}), item({ harness: "codex" })])).toBe("active");
  });

  it("is off when any copy is switched off", () => {
    expect(status([item({}), item({ harness: "codex", enabled: false })])).toBe(
      "off",
    );
  });

  it("reports a broken link over a switch — the file is gone either way", () => {
    const broken = item({
      enabled: false,
      fileState: { state: "symlink", target: "/gone", broken: true },
    });
    expect(status([broken])).toBe("broken");
  });

  it("says nothing about a link that still points somewhere", () => {
    const linked = item({
      fileState: { state: "symlink", target: "/src/deploy", broken: false },
    });
    expect(status([linked])).toBe("active");
  });
});
