import { describe, expect, it } from "vitest";
import {
  kindLabel,
  STATE_LABELS,
  scopeName,
  scopePath,
  TOOL_NAMES,
  toolName,
} from "./labels";

describe("labels", () => {
  it("pluralizes kind labels by count", () => {
    expect(kindLabel("skill")).toBe("Skill");
    expect(kindLabel("skill", 3)).toBe("Skills");
    expect(kindLabel("mcp-server", 0)).toBe("MCP servers");
  });

  it("names scopes by folder, global by name", () => {
    expect(scopeName({ scope: "global" })).toBe("Global");
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
});
