import type { AuditView } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  APPLY_CONFIRM_LABEL,
  APPLY_DIALOG_BODY,
  APPLY_DIALOG_TITLE,
  removeLeftBehindLabel,
} from "@/lib/copy";
import type { MergedDriftRow } from "@/lib/drift-merge";
import { kindLabel } from "@/lib/labels";

/** The last look before anything is written: every line this will do, and
 *  the one opt-in extra — clearing out items nothing declares any more. */
export function ApplyDialog({
  open,
  onOpenChange,
  view,
  orphans,
  busy,
  removeOrphans,
  onRemoveOrphansChange,
  onApply,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  view: AuditView;
  orphans: MergedDriftRow[];
  busy: boolean;
  removeOrphans: boolean;
  onRemoveOrphansChange: (value: boolean) => void;
  onApply: () => void;
}) {
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={APPLY_DIALOG_TITLE}
      description={APPLY_DIALOG_BODY}
      confirmLabel={APPLY_CONFIRM_LABEL}
      busy={busy}
      onConfirm={onApply}
    >
      <div className="flex flex-col gap-3">
        <div className="flex max-h-48 flex-col gap-1 overflow-y-auto rounded-md border bg-muted/40 p-3">
          {view.plan.map((line) => (
            <p key={line} className="break-words text-xs text-muted-foreground">
              {line}
            </p>
          ))}
          {removeOrphans
            ? orphans.map((group) => (
                <p
                  key={`rm:${group.kind}:${group.name}:${group.state}`}
                  className="break-words text-xs text-muted-foreground"
                >
                  Remove {kindLabel(group.kind).toLowerCase()} {group.name} —
                  nothing asks for it any more
                </p>
              ))
            : null}
        </div>
        {orphans.length > 0 ? (
          <div className="flex items-center gap-2 text-sm">
            <Checkbox
              id="remove-orphans"
              checked={removeOrphans}
              onCheckedChange={(checked) =>
                onRemoveOrphansChange(checked === true)
              }
            />
            <Label htmlFor="remove-orphans" className="font-normal">
              {removeLeftBehindLabel(orphans.length)}
            </Label>
          </div>
        ) : null}
      </div>
    </ConfirmDialog>
  );
}
