import { describe, expect, it } from "vitest";
import type { DriftRow } from "@/bindings";
import {
  driftDetail,
  hookDisplayName,
  kindLabel,
  SEVERITY_BADGES,
  STATE_BADGES,
  STATE_LABELS,
  scopeName,
  scopePath,
  skipReasonShort,
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

  it("shows a hook's trailing name and falls back to the whole id", () => {
    expect(hookDisplayName("Notification:permission_prompt:tmux-bell")).toBe(
      "tmux-bell",
    );
    expect(hookDisplayName("PreToolUse:*:claude-hook")).toBe("claude-hook");
    expect(hookDisplayName("guard")).toBe("guard");
  });

  it("drops drift detail that only restates the state pill", () => {
    const row = (over: Partial<DriftRow>): DriftRow => ({
      kind: "skill",
      name: "x",
      harness: "claude",
      scope: { scope: "global" },
      state: "stale",
      detail: "",
      ...over,
    });
    expect(
      driftDetail(
        row({ state: "stale", detail: "newer content is available" }),
      ),
    ).toBeNull();
    expect(
      driftDetail(row({ state: "missing", detail: "not installed yet" })),
    ).toBeNull();
    expect(driftDetail(row({ detail: "" }))).toBeNull();
    expect(
      driftDetail(
        row({ state: "conflict", detail: "both a symlink and a real file" }),
      ),
    ).toBe("both a symlink and a real file");
  });

  it("shortens a known skip reason and falls back for unknown ones", () => {
    expect(
      skipReasonShort(
        "the plugin's own files are not readable here — a declared plugin is one switch in a settings file until it is installed",
      ),
    ).toBe("can't be fully checked until they're installed");
    expect(skipReasonShort("some new engine sentence")).toBe(
      "can't be fully checked here yet",
    );
  });
});
