import type { HarnessId, ItemKind } from "@/bindings";
import { ToolBadge } from "@/components/tool-badge";
import { Badge } from "@/components/ui/badge";
import { kindLabel } from "@/lib/labels";

/**
 * What a thing is, and which tools load it — the Library's treatment,
 * reused anywhere a row needs to say the same two facts. One grey chip for
 * the kind, then a mark per tool, so the tools stay pickable out of a row at
 * a glance instead of hiding inside "Skill · Codex, Pi".
 */
export function KindToolChips({
  kind,
  harnesses,
}: {
  kind: ItemKind;
  harnesses: HarnessId[];
}) {
  return (
    <span className="flex shrink-0 items-center gap-1.5">
      <Badge variant="outline">{kindLabel(kind)}</Badge>
      {harnesses.map((harness) => (
        <ToolBadge key={harness} harness={harness} compact />
      ))}
    </span>
  );
}
