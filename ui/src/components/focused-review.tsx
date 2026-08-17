import { useState } from "react";
import type { DismissReason, ItemSafety } from "@/bindings";
import { FindingLine } from "@/components/safety-findings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  FOCUSED_ALL_DONE,
  FOCUSED_ALL_DONE_BODY,
  FOCUSED_SKIP,
  focusedBody,
  focusedProgress,
  NO_SOURCE_TO_TRUST,
  REASON_HELP,
  REASON_LABELS,
  REASON_ORDER,
} from "@/lib/copy-decisions";
import { reviewQueue, stillOpen } from "@/lib/focused-review";
import { kindLabel, toolName } from "@/lib/labels";
import type { EvidenceGroup } from "@/lib/reviewable";

/**
 * One finding at a time. Twenty plugins tripping one rule are twenty
 * different pieces of content, and the honest way through them is to look
 * at each — so this walks the scope's open evidence worst-first, showing
 * one piece with its item, its finding and the three reasons, and moves on
 * as each is decided or skipped. The queue is taken when the walk starts;
 * a step decided from elsewhere in the meantime is skipped, not re-asked.
 */
export function FocusedReview({
  open,
  onOpenChange,
  rows,
  projectScope,
  busy,
  onDismiss,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The scope's live safety rows — re-read on every step. */
  rows: ItemSafety[];
  projectScope: boolean;
  busy: boolean;
  onDismiss: (tokens: string[], reason: DismissReason) => void;
}) {
  // The walk mounts when the dialog opens and unmounts when it closes, so
  // the queue it takes on mount is frozen for exactly one walk.
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {open ? (
        <Walk
          rows={rows}
          projectScope={projectScope}
          busy={busy}
          onDismiss={onDismiss}
          onClose={() => onOpenChange(false)}
        />
      ) : null}
    </Dialog>
  );
}

function Walk({
  rows,
  projectScope,
  busy,
  onDismiss,
  onClose,
}: {
  rows: ItemSafety[];
  projectScope: boolean;
  busy: boolean;
  onDismiss: (tokens: string[], reason: DismissReason) => void;
  onClose: () => void;
}) {
  const [queue] = useState<EvidenceGroup[]>(() => reviewQueue(rows));
  const [index, setIndex] = useState(0);
  // Steps decided elsewhere since the queue was taken are stepped over.
  let at = index;
  while (at < queue.length && !stillOpen(queue[at], rows)) at += 1;
  const step = queue[at];

  if (!step) {
    return (
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{FOCUSED_ALL_DONE}</DialogTitle>
          <DialogDescription>{FOCUSED_ALL_DONE_BODY}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button onClick={onClose}>Done</Button>
        </DialogFooter>
      </DialogContent>
    );
  }

  const first = step.items[0];
  const tools = [...new Set(step.items.map((item) => toolName(item.harness)))];
  const decide = (reason: DismissReason) => {
    onDismiss(step.tokens, reason);
    setIndex(at + 1);
  };
  return (
    <DialogContent>
      <DialogHeader>
        <DialogTitle>{focusedProgress(at + 1, queue.length)}</DialogTitle>
        <DialogDescription>{focusedBody(projectScope)}</DialogDescription>
      </DialogHeader>
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-medium">{first.name}</span>
          <Badge variant="outline" className="font-normal">
            {kindLabel(first.kind)} · {tools.join(", ")}
          </Badge>
        </div>
        <FindingLine finding={step.finding} />
        <div className="flex flex-col gap-1.5">
          {REASON_ORDER.map((reason) => {
            const disabled =
              reason === "trusted-source" && !step.canTrustSource;
            return (
              <button
                key={reason}
                type="button"
                disabled={busy || disabled}
                onClick={() => decide(reason)}
                className="flex cursor-pointer flex-col items-start gap-0.5 rounded-lg border px-3 py-2.5 text-left hover:bg-muted/40 disabled:cursor-not-allowed disabled:opacity-60"
              >
                <span className="text-sm font-medium">
                  {REASON_LABELS[reason]}
                </span>
                <span className="text-[13px] text-muted-foreground">
                  {disabled ? NO_SOURCE_TO_TRUST : REASON_HELP[reason]}
                </span>
              </button>
            );
          })}
        </div>
      </div>
      <DialogFooter>
        <Button
          variant="outline"
          disabled={busy}
          onClick={() => setIndex(at + 1)}
        >
          {FOCUSED_SKIP}
        </Button>
      </DialogFooter>
    </DialogContent>
  );
}
