import { describe, expect, it } from "vitest";
import { isSearchShortcutKey } from "./search-shortcut";

describe("isSearchShortcutKey", () => {
  it("fires for / typed outside any field", () => {
    expect(isSearchShortcutKey("/", null)).toBe(true);
    expect(isSearchShortcutKey("/", { tagName: "DIV" })).toBe(true);
    expect(isSearchShortcutKey("/", { tagName: "BUTTON" })).toBe(true);
  });

  it("ignores keys other than /", () => {
    expect(isSearchShortcutKey("a", null)).toBe(false);
    expect(isSearchShortcutKey("Enter", null)).toBe(false);
  });

  it("does not steal / while typing in a field", () => {
    expect(isSearchShortcutKey("/", { tagName: "INPUT" })).toBe(false);
    expect(isSearchShortcutKey("/", { tagName: "TEXTAREA" })).toBe(false);
    expect(isSearchShortcutKey("/", { tagName: "SELECT" })).toBe(false);
    expect(
      isSearchShortcutKey("/", { tagName: "DIV", isContentEditable: true }),
    ).toBe(false);
  });
});
