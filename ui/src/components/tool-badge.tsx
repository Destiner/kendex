import type { HarnessId } from "@/bindings";
import { ToolIcon } from "@/components/tool-icon";
import { Badge } from "@/components/ui/badge";
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
 * without reading it. Colour is the only thing carrying the tool's identity
 * here, so the label is never dropped — the hue speeds up a scan, it doesn't
 * replace the name.
 */
export function ToolBadge({
  harness,
  className,
}: {
  harness: HarnessId;
  className?: string;
}) {
  return (
    <Badge className={cn("border-transparent", TOOL_CHIP[harness], className)}>
      <ToolIcon harness={harness} className="size-3" />
      {toolName(harness)}
    </Badge>
  );
}
