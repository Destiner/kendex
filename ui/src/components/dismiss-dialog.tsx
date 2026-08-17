import { useState } from "react";
import type { DismissReason } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import {
  DISMISS_CONFIRM,
  DISMISS_TITLE,
  dismissBody,
  dismissManyBody,
  dismissManyTitle,
  NO_SOURCE_TO_TRUST,
  REASON_HELP,
  REASON_LABELS,
  REASON_ORDER,
} from "@/lib/copy-decisions";
import { cn } from "@/lib/utils";

/**
 * Dismissing a finding asks one thing: why. The reasons are a closed list,
 * each a claim about the content rather than about the person deciding —
 * a project's decisions travel with the repository, so a teammate has to
 * be able to read the reason and agree. The body says which file the
 * decision lands in, the same honesty the accept dialog has.
 */
export function DismissDialog({
  open,
  onOpenChange,
  count,
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
      onOpenChange={onOpenChange}
      title={count > 1 ? dismissManyTitle(count) : DISMISS_TITLE}
      description={[
        count > 1 ? dismissManyBody : null,
        dismissBody(projectScope),
      ]
        .filter(Boolean)
        .join(" ")}
      confirmLabel={DISMISS_CONFIRM}
      busy={busy}
      onConfirm={() => onConfirm(reason)}
    >
      <fieldset className="flex flex-col gap-1.5">
        <legend className="mb-1.5 text-sm font-medium">Why?</legend>
        {REASON_ORDER.map((option) => {
          const disabled = option === "trusted-source" && !canTrustSource;
          const selected = reason === option;
          return (
            <label
              key={option}
              className={cn(
                "flex cursor-pointer items-start gap-3 rounded-lg border px-3 py-2.5",
                selected ? "border-foreground/40 bg-muted/40" : "border-border",
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
