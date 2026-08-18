import type { HarnessId } from "@/bindings";
import { ToolIcon } from "@/components/tool-icon";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { toolName } from "@/lib/labels";
import { cn } from "@/lib/utils";

// Written out per tool rather than composed from the id: Tailwind only
// emits classes it can see as whole strings in the source.
// A tinted fill rather than an outline: a row with five outlined chips reads
// as five buttons, where five washes of colour read as five labels.
const TOOL_CHIP: Record<HarnessId, string> = {
  claude: "bg-tool-claude/12 text-tool-claude",
  codex: "bg-tool-codex/12 text-tool-codex",
  opencode: "bg-tool-opencode/12 text-tool-opencode",
  cursor: "bg-tool-cursor/12 text-tool-cursor",
  pi: "bg-tool-pi/12 text-tool-pi",
  gemini: "bg-tool-gemini/12 text-tool-gemini",
  copilot: "bg-tool-copilot/12 text-tool-copilot",
};

/**
 * The tool a thing is installed for, as a chip you can pick out of a row
 * without reading it.
 *
 * `compact` drops the name and keeps the mark. In a table every row carries
 * the same five or six tools, so the names are a column of repeated words
 * pushing the columns that differ off the screen — the logo and its hue
 * already tell them apart, and the name arrives on hover. Where a tool is
 * stated once rather than listed — a package's own details — the name stays
 * written out, since there is nothing there to scan past.
 */
export function ToolBadge({
  harness,
  compact,
  className,
}: {
  harness: HarnessId;
  compact?: boolean;
  className?: string;
}) {
  if (compact) {
    return (
      <Tooltip>
        <TooltipTrigger
          render={
            <Badge
              aria-label={toolName(harness)}
              className={cn(
                "border-transparent px-1.5",
                TOOL_CHIP[harness],
                className,
              )}
            >
              <ToolIcon harness={harness} className="size-3.5" />
            </Badge>
          }
        />
        <TooltipContent>{toolName(harness)}</TooltipContent>
      </Tooltip>
    );
  }
  return (
    <Badge className={cn("border-transparent", TOOL_CHIP[harness], className)}>
      <ToolIcon harness={harness} className="size-3" />
      {toolName(harness)}
    </Badge>
  );
}
