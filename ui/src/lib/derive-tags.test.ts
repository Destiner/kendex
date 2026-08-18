import { describe, expect, it } from "vitest";
import type { ObservedItem } from "@/bindings";
import { filterItems, groupItems } from "./derive";

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

describe("filterItems by tag", () => {
  it("keeps only items carrying the tag", () => {
    const items = [
      item({ name: "reviewer", tags: ["review", "testing"] }),
      item({ name: "shipper", tags: ["release"] }),
      item({ name: "untagged" }),
    ];
    const kept = filterItems(items, { scope: "all", tag: "review" });
    expect(kept.map((i) => i.name)).toEqual(["reviewer"]);
  });

  it("keeps everything when no tag is asked for", () => {
    const items = [item({ name: "a" }), item({ name: "b", tags: ["docs"] })];
    expect(filterItems(items, { scope: "all" })).toHaveLength(2);
  });
});

describe("groupItems tags", () => {
  // Two installations of one item can be copies that disagree; what the
  // item is for is everything either of them claims, said once.
  it("unions the tags across installations without repeating one", () => {
    const group = groupItems([
      item({ harness: "claude", tags: ["review", "testing"] }),
      item({ harness: "pi", tags: ["testing", "docs"] }),
    ])[0];
    expect(group.tags).toEqual(["review", "testing", "docs"]);
  });

  it("has no tags when nothing claimed any", () => {
    expect(groupItems([item({})])[0].tags).toEqual([]);
  });
});
