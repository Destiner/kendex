import { useState } from "react";
import type { DismissReason } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import {
  IGNORE_CONFIRM,
  IGNORE_TITLE,
  ignoreBody,
  ignoreManyBody,
  ignoreManyTitle,
  NO_SOURCE_TO_TRUST,
  REASON_HELP,
  REASON_LABELS,
  REASON_ORDER,
} from "@/lib/copy-decisions";
import { cn } from "@/lib/utils";

/**
 * Ignoring a finding asks one thing: why. The reasons are a closed list,
 * each a claim about the content rather than about the person deciding —
 * a project's decisions travel with the repository, so a teammate has to
 * be able to read the reason and agree. The body says which file the
 * decision lands in, the same honesty the accept dialog has.
 */
export function IgnoreDialog({
  open,
  onOpenChange,
  count,
  subject,
  finding,
  projectScope,
  canTrustSource,
  busy,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** How many installations the decision covers — the same bytes seen
   *  through several tools. One is the common case. */
  count: number;
  /** What is being ruled on, named the way the row names it. */
  subject: string;
  /** The finding's own words — the dialog is opened from a row that only
   *  showed a headline, so the claim is restated where the call is made. */
  finding: string;
  projectScope: boolean;
  /** Whether every installation can name where its content came from;
   *  trusting a source needs one to trust. */
  canTrustSource: boolean;
  busy: boolean;
  onConfirm: (reason: DismissReason) => void;
}) {
  const [reason, setReason] = useState<DismissReason>("wrong-call");
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={(next) => {
        // A cancelled pick must not pre-answer the next question.
        if (!next) setReason("wrong-call");
        onOpenChange(next);
      }}
      title={count > 1 ? ignoreManyTitle(count) : IGNORE_TITLE}
      description={[count > 1 ? ignoreManyBody : null, ignoreBody(projectScope)]
        .filter(Boolean)
        .join(" ")}
      confirmLabel={IGNORE_CONFIRM}
      busy={busy}
      onConfirm={() => onConfirm(reason)}
    >
      <p className="text-sm">
        <span className="font-medium">{subject}</span>
        <span className="text-muted-foreground"> — {finding}</span>
      </p>
      <fieldset className="divide-y overflow-hidden rounded-lg border">
        <legend className="sr-only">Why?</legend>
        {REASON_ORDER.map((option) => {
          const disabled = option === "trusted-source" && !canTrustSource;
          const selected = reason === option;
          return (
            <label
              key={option}
              className={cn(
                "flex cursor-pointer items-start gap-3 px-3 py-2.5",
                selected ? "bg-muted/60" : "hover:bg-muted/30",
                disabled && "cursor-not-allowed opacity-60",
              )}
            >
              <input
                type="radio"
                name="dismiss-reason"
                value={option}
                checked={selected}
                disabled={disabled}
                onChange={() => setReason(option)}
                className="mt-1 accent-foreground"
              />
              <span className="flex min-w-0 flex-col gap-0.5">
                <span className="text-sm font-medium">
                  {REASON_LABELS[option]}
                </span>
                <span className="text-[13px] text-muted-foreground">
                  {disabled ? NO_SOURCE_TO_TRUST : REASON_HELP[option]}
                </span>
              </span>
            </label>
          );
        })}
      </fieldset>
    </ConfirmDialog>
  );
}
