import type { HarnessId } from "@/bindings";
import { cn } from "@/lib/utils";

// Drawn here rather than shipped as brand assets: these are simple,
// recognisable marks in each tool's own hue, not the vendors' logos. That
// keeps the set visually consistent at 16px, avoids passing anyone's
// trademark off as ours, and means the icon and the tool's colour can never
// disagree — both come from the same place.
const PATHS: Record<HarnessId, React.ReactNode> = {
  // Anthropic's burst, as an eight-armed asterisk.
  claude: (
    <g strokeWidth="1.8" strokeLinecap="round">
      <path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6L5.6 18.4" />
    </g>
  ),
  // A knot of overlapping rings, the shape of OpenAI's mark without being it.
  codex: (
    <g strokeWidth="1.6">
      <circle cx="12" cy="8.5" r="4" />
      <circle cx="8.5" cy="14.5" r="4" />
      <circle cx="15.5" cy="14.5" r="4" />
    </g>
  ),
  // A terminal: OpenCode lives in one.
  opencode: (
    <g strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <path d="M7.5 9.5l3 2.5-3 2.5M13 15h4" />
    </g>
  ),
  // Cursor's cube, in outline.
  cursor: (
    <g strokeWidth="1.6" strokeLinejoin="round">
      <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9L12 3z" />
      <path d="M12 12l8-4.5M12 12v9M12 12L4 7.5" />
    </g>
  ),
  // The letter itself — nothing reads as Pi faster.
  pi: (
    <g strokeWidth="1.9" strokeLinecap="round">
      <path d="M4.5 7h15M9 7v10M16 7v7.5c0 1.4.8 2.5 2.2 2.5" />
    </g>
  ),
  // Gemini's four-point spark.
  gemini: (
    <g strokeWidth="1.6" strokeLinejoin="round">
      <path d="M12 2.5c0 5 4.5 9.5 9.5 9.5-5 0-9.5 4.5-9.5 9.5 0-5-4.5-9.5-9.5-9.5 5 0 9.5-4.5 9.5-9.5z" />
    </g>
  ),
  // Copilot: a small pilot at the controls.
  copilot: (
    <g strokeWidth="1.6" strokeLinejoin="round">
      <rect x="3" y="8.5" width="18" height="10" rx="4" />
      <path d="M8.5 12.5v2M15.5 12.5v2M12 8.5V5" />
      <circle cx="12" cy="4" r="1.4" />
    </g>
  ),
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
  /** Drawn in grey instead: a tool that isn't installed here. */
  muted?: boolean;
}) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      aria-hidden="true"
      className={cn(
        "size-4 shrink-0",
        muted ? "text-muted-foreground" : TINT[harness],
        className,
      )}
    >
      {PATHS[harness]}
    </svg>
  );
}
