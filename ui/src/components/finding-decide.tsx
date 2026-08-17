import { useState } from "react";
import type { DismissReason } from "@/bindings";
import { DismissDialog } from "@/components/dismiss-dialog";
import { StatusLine } from "@/components/status-note";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DISMISS_LABEL,
  earlierDecisionNote,
  UNDECIDABLE_HERE,
} from "@/lib/copy-decisions";
import { abbreviateHome } from "@/lib/drift-merge";
import { toolName } from "@/lib/labels";
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
  if (group.tokens.length === 0) {
    return (
      <span className="shrink-0 text-xs text-muted-foreground">
        {UNDECIDABLE_HERE}
      </span>
    );
  }
  return (
    <>
      <Button
        size="sm"
        variant="outline"
        className="shrink-0"
        disabled={busy}
        aria-label={`Dismiss the finding on ${group.items[0].name}`}
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
  // A hook's full id — event, matcher, script — is what tells seven hooks
  // in one settings file apart; the short display name would not.
  const first = group.items[0];
  const name = first.name;
  const tools = [...new Set(group.items.map((item) => toolName(item.harness)))];
  return (
    <div className="flex items-center gap-2.5 py-1.5">
      <Badge
        variant="outline"
        className="max-w-full shrink-0 truncate font-normal"
      >
        {name}
      </Badge>
      <span className="flex min-w-0 flex-1 flex-col text-xs text-muted-foreground">
        <span className="truncate">
          {tools.join(", ")}
          {" · "}
          <span className="font-mono">
            {abbreviateHome(group.finding.location)}
          </span>
        </span>
        {group.earlier ? (
          <StatusLine tone="info">
            {earlierDecisionNote(group.earlier)}
          </StatusLine>
        ) : null}
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
