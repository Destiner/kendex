import { describe, expect, it } from "vitest";
import type { HookEvent } from "@/bindings";
import { matchingEvents } from "./hook-events";

const EVENTS: HookEvent[] = [
  { name: "PreToolUse", fires: "Before the agent runs a tool" },
  { name: "PostToolUse", fires: "After a tool returns" },
  { name: "SessionStart", fires: "A session starts" },
];

describe("matchingEvents", () => {
  it("returns everything when nothing has been typed", () => {
    expect(matchingEvents(EVENTS, "  ")).toHaveLength(3);
  });

  it("matches the name, whatever case it is typed in", () => {
    expect(matchingEvents(EVENTS, "pretool").map((e) => e.name)).toEqual([
      "PreToolUse",
    ]);
  });

  /** The point of the filter: finding an event you cannot name. */
  it("matches what the event fires on, not only its name", () => {
    expect(matchingEvents(EVENTS, "session").map((e) => e.name)).toEqual([
      "SessionStart",
    ]);
    expect(matchingEvents(EVENTS, "runs a tool").map((e) => e.name)).toEqual([
      "PreToolUse",
    ]);
  });

  it("returns nothing when nothing matches", () => {
    expect(matchingEvents(EVENTS, "webhook")).toEqual([]);
  });
});
