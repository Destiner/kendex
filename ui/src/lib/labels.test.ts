import { describe, expect, it } from "vitest";
import {
  kindLabel,
  SEVERITY_BADGES,
  STATE_BADGES,
  STATE_LABELS,
  scopeName,
  scopePath,
  TOOL_NAMES,
  toolName,
  VERDICT_BADGES,
} from "./labels";

describe("labels", () => {
  it("pluralizes kind labels by count", () => {
    expect(kindLabel("skill")).toBe("Skill");
    expect(kindLabel("skill", 3)).toBe("Skills");
    expect(kindLabel("mcp-server", 0)).toBe("MCP servers");
  });

  it("names scopes by folder, global by name", () => {
    expect(scopeName({ scope: "global" })).toBe("Personal");
    expect(scopeName({ scope: "project", root: "/home/x/acme-web" })).toBe(
      "acme-web",
    );
    expect(scopePath({ scope: "global" })).toBeNull();
    expect(scopePath({ scope: "project", root: "/home/x/acme-web" })).toBe(
      "/home/x/acme-web",
    );
  });

  it("keeps human copy free of internal jargon", () => {
    const copy = [...Object.values(STATE_LABELS), ...Object.values(TOOL_NAMES)]
      .join(" ")
      .toLowerCase();
    for (const banned of ["drift", "unmanaged", "orphan", "harness", "scope"]) {
      expect(copy).not.toContain(banned);
    }
    expect(toolName("claude")).toBe("Claude Code");
  });

  it("maps drift states to the badge tone that matches their urgency", () => {
    expect(STATE_BADGES.conflict).toBe("warning");
    expect(STATE_BADGES.stale).toBe("info");
    expect(STATE_BADGES.missing).toBe("info");
    expect(STATE_BADGES.orphaned).toBe("outline");
    expect(STATE_BADGES.unmanaged).toBe("secondary");
  });

  it("maps severity to the badge tone that matches how serious it is", () => {
    expect(SEVERITY_BADGES.critical).toBe("critical");
    expect(SEVERITY_BADGES.high).toBe("warning");
    expect(SEVERITY_BADGES.medium).toBe("info");
    expect(SEVERITY_BADGES.low).toBe("secondary");
  });

  it("maps a safety verdict to the badge tone that matches its outcome", () => {
    expect(VERDICT_BADGES.block).toBe("critical");
    expect(VERDICT_BADGES.warn).toBe("warning");
    expect(VERDICT_BADGES.clean).toBe("good");
  });
});
