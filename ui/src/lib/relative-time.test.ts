import { describe, expect, it } from "vitest";
import { relativeTime } from "./relative-time";

describe("relativeTime", () => {
  it("reads as just now under a minute", () => {
    expect(relativeTime(0, 45_000)).toBe("just now");
  });

  it("rounds to whole minutes", () => {
    expect(relativeTime(0, 2 * 60_000)).toBe("2m ago");
  });

  it("rounds to whole hours once past 60 minutes", () => {
    expect(relativeTime(0, 3 * 60 * 60_000)).toBe("3h ago");
  });

  it("rounds to whole days once past 24 hours", () => {
    expect(relativeTime(0, 2 * 24 * 60 * 60_000)).toBe("2d ago");
  });
});
