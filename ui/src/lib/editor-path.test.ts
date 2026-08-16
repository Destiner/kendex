import { describe, expect, it } from "vitest";
import { editorOpenPath } from "./editor-path";

describe("editorOpenPath", () => {
  it("strips SKILL.md so the editor opens the whole skill folder", () => {
    expect(editorOpenPath("/home/user/.claude/skills/foo/SKILL.md")).toBe(
      "/home/user/.claude/skills/foo",
    );
  });

  it("strips it behind a Windows separator too", () => {
    expect(editorOpenPath("C:\\Users\\u\\.claude\\skills\\foo\\SKILL.md")).toBe(
      "C:\\Users\\u\\.claude\\skills\\foo",
    );
  });

  it("leaves non-skill paths unchanged", () => {
    expect(editorOpenPath("/home/user/.claude/hooks/pre-commit.sh")).toBe(
      "/home/user/.claude/hooks/pre-commit.sh",
    );
  });
});
