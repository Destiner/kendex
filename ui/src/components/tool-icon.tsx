import ClaudeMark from "@/assets/tools/claude.svg?react";
import CodexMark from "@/assets/tools/codex.svg?react";
import CopilotMark from "@/assets/tools/copilot.svg?react";
import CursorMark from "@/assets/tools/cursor.svg?react";
import GeminiMark from "@/assets/tools/gemini.svg?react";
import OpencodeMark from "@/assets/tools/opencode.svg?react";
import PiMark from "@/assets/tools/pi.svg?react";
import type { HarnessId } from "@/bindings";
import { cn } from "@/lib/utils";

// The vendors' own marks, taken from their own sites and brand kits —
// provenance and the exact edits are in assets/tools/SOURCES.md. The
// single-colour marks carry `fill="currentColor"` in the file, so the
// tool's hue still comes from the same `--tool-*` token as the badges and
// they survive dark mode; Gemini keeps its own gradient, because the
// gradient is the mark.
const MARKS: Record<HarnessId, React.FC<React.SVGProps<SVGSVGElement>>> = {
  claude: ClaudeMark,
  codex: CodexMark,
  opencode: OpencodeMark,
  cursor: CursorMark,
  pi: PiMark,
  gemini: GeminiMark,
  copilot: CopilotMark,
};

const TINT: Record<HarnessId, string> = {
  claude: "text-tool-claude",
  codex: "text-tool-codex",
  opencode: "text-tool-opencode",
  cursor: "text-tool-cursor",
  pi: "text-tool-pi",
  gemini: "text-tool-gemini",
  copilot: "text-tool-copilot",
};

/** A tool's mark, in the tool's own colour. Decorative — every place this
 *  appears also names the tool in text. */
export function ToolIcon({
  harness,
  className,
  muted = false,
}: {
  harness: HarnessId;
  className?: string;
  /** Drawn in grey instead: a tool that isn't installed here. `grayscale`
   *  is for the marks that ignore `currentColor` (Gemini's gradient). */
  muted?: boolean;
}) {
  const Mark = MARKS[harness];
  return (
    <Mark
      aria-hidden="true"
      preserveAspectRatio="xMidYMid meet"
      className={cn(
        "size-4 shrink-0",
        muted ? "text-muted-foreground grayscale opacity-80" : TINT[harness],
        className,
      )}
    />
  );
}
