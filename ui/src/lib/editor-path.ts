// A skill's path may point at its SKILL.md file rather than its folder;
// opening just the file in an editor leaves the rest of the skill's files
// out of the workspace, so the Editor action opens the containing folder
// instead. Every other kind's path is already the right thing to open.
export function editorOpenPath(path: string): string {
  const match = /[\\/]SKILL\.md$/.exec(path);
  return match ? path.slice(0, match.index) : path;
}

// A finding names where it fired, which for a text rule is a file plus the
// line it matched on ("…/pr-create.sh:32"). Anything opening the file wants
// the file, not the line marker; a Windows drive letter is not one.
export function fileOfLocation(location: string): string {
  return location.replace(/:\d+$/, "");
}
