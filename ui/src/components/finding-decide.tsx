import { useState } from "react";
import type { DismissReason } from "@/bindings";
import { DismissDialog } from "@/components/dismiss-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DISMISS_LABEL } from "@/lib/copy-decisions";
import { abbreviateHome } from "@/lib/drift-merge";
import { hookDisplayName, toolName } from "@/lib/labels";
import type { EvidenceGroup } from "@/lib/reviewable";

/**
 * The button that rules on one piece of evidence — the same bytes carrying
 * the same finding, however many tools read them. It opens the reason
 * dialog and sends exactly this group's tokens; nothing about the row it
 * sits on can widen that.
 */
export function DismissButton({
  group,
  projectScope,
  busy,
  onDismiss,
}: {
  group: EvidenceGroup;
  projectScope: boolean;
  busy: boolean;
  onDismiss: (tokens: string[], reason: DismissReason) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        size="sm"
        variant="outline"
        className="shrink-0"
        disabled={busy}
        onClick={(event) => {
          event.stopPropagation();
          setOpen(true);
        }}
      >
        {DISMISS_LABEL}
      </Button>
      <DismissDialog
        open={open}
        onOpenChange={setOpen}
        count={group.tokens.length}
        projectScope={projectScope}
        canTrustSource={group.canTrustSource}
        busy={busy}
        onConfirm={(reason) => {
          setOpen(false);
          onDismiss(group.tokens, reason);
        }}
      />
    </>
  );
}

/** One evidence group as a line a person can rule on: what it is on, where
 *  it is, and the button. The same file installed for three tools is one
 *  line naming all three, because that is one decision. */
export function EvidenceLine({
  group,
  projectScope,
  busy,
  onDismiss,
}: {
  group: EvidenceGroup;
  projectScope: boolean;
  busy: boolean;
  onDismiss: (tokens: string[], reason: DismissReason) => void;
}) {
  const first = group.items[0];
  const name = first.kind === "hook" ? hookDisplayName(first.name) : first.name;
  const tools = [...new Set(group.items.map((item) => toolName(item.harness)))];
  return (
    <div className="flex items-center gap-2.5 py-1.5">
      <Badge
        variant="outline"
        className="max-w-full shrink-0 truncate font-normal"
      >
        {name}
      </Badge>
      <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">
        {tools.join(", ")}
        {" · "}
        <span className="font-mono">
          {abbreviateHome(group.finding.location)}
        </span>
      </span>
      <DismissButton
        group={group}
        projectScope={projectScope}
        busy={busy}
        onDismiss={onDismiss}
      />
    </div>
  );
}
