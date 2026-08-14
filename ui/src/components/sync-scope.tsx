import { useState } from "react";
import type { AuditView, DriftRow } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { SafetyFindings } from "@/components/safety-findings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  kindLabel,
  STATE_BADGES,
  STATE_LABELS,
  scopeName,
  scopePath,
  toolName,
} from "@/lib/labels";

export function SyncScopeCard({
  view,
  busy,
  onApply,
  onAdopt,
}: {
  view: AuditView;
  busy: boolean;
  onApply: (removeOrphans: boolean) => void;
  onAdopt: (
    kind: DriftRow["kind"],
    name: string,
    harness: DriftRow["harness"],
  ) => void;
}) {
  const [reviewOpen, setReviewOpen] = useState(false);
  const orphans = view.drift.filter((row) => row.state === "orphaned");
  // With nothing else to fix, removing left-behind items is the only
  // change on offer — defaulting the checkbox on keeps it reachable.
  const orphansOnly = orphans.length > 0 && view.plan.length === 0;
  const [removeOrphans, setRemoveOrphans] = useState(orphansOnly);
  const path = scopePath(view.scope);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-start gap-3 text-base">
          <div className="min-w-0">
            <span className="break-all">{scopeName(view.scope)}</span>
            {path ? (
              <p className="truncate font-mono text-xs font-normal text-muted-foreground">
                {path}
              </p>
            ) : null}
          </div>
          {view.plan.length > 0 || orphans.length > 0 ? (
            <Button
              size="sm"
              className="ml-auto shrink-0"
              disabled={busy}
              onClick={() => {
                if (orphansOnly) setRemoveOrphans(true);
                setReviewOpen(true);
              }}
            >
              Review changes…
            </Button>
          ) : null}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {view.drift.map((row) => (
          <div
            key={`${row.kind}:${row.name}:${row.harness}`}
            className="flex flex-wrap items-start gap-2 text-sm"
          >
            <Badge variant={STATE_BADGES[row.state]}>
              {STATE_LABELS[row.state]}
            </Badge>
            <span className="font-semibold">{row.name}</span>
            <span className="text-muted-foreground">
              {kindLabel(row.kind)} · {toolName(row.harness)}
            </span>
            <span className="min-w-0 flex-1 basis-full break-words text-muted-foreground">
              {row.detail}
            </span>
            {row.state === "unmanaged" ? (
              <Button
                size="sm"
                variant="outline"
                disabled={busy}
                onClick={() => onAdopt(row.kind, row.name, row.harness)}
              >
                Start managing
              </Button>
            ) : null}
          </div>
        ))}
        {view.notes.map((note) => (
          <p key={note} className="text-xs text-muted-foreground">
            {note}
          </p>
        ))}
        {view.warnings.map((warning) => (
          <p
            key={`${warning.name}-${warning.harness ?? ""}-${warning.message}`}
            className="text-xs text-muted-foreground"
          >
            {warning.name}
            {warning.harness ? ` (${warning.harness})` : ""}: {warning.message}
            {warning.remediation ? ` — fix: ${warning.remediation}` : ""}
          </p>
        ))}
        <SafetyFindings rows={view.safety} />
      </CardContent>
      <ConfirmDialog
        open={reviewOpen}
        onOpenChange={setReviewOpen}
        title="Apply these changes?"
        description="vstack will update the files it manages. Nothing else is touched."
        confirmLabel="Apply changes"
        busy={busy}
        onConfirm={() => {
          onApply(removeOrphans);
          setReviewOpen(false);
        }}
      >
        <div className="space-y-3">
          <div className="max-h-48 space-y-1 overflow-y-auto rounded-md border bg-muted/40 p-3">
            {view.plan.map((line) => (
              <p
                key={line}
                className="break-words text-xs text-muted-foreground"
              >
                {line}
              </p>
            ))}
            {removeOrphans
              ? orphans.map((row) => (
                  <p
                    key={`rm:${row.kind}:${row.name}:${row.harness}`}
                    className="break-words text-xs text-muted-foreground"
                  >
                    Remove {kindLabel(row.kind).toLowerCase()} {row.name} — left
                    behind
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
                  setRemoveOrphans(checked === true)
                }
              />
              <Label htmlFor="remove-orphans" className="font-normal">
                {orphansOnly
                  ? "Remove items that were left behind"
                  : "Also remove items that were left behind"}
              </Label>
            </div>
          ) : null}
        </div>
      </ConfirmDialog>
    </Card>
  );
}
