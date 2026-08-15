import { useState } from "react";
import type { AuditView, DriftRow } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { ConfirmDialog } from "@/components/confirm-dialog";
import {
  BlockedFindings,
  SafetyCleanSummary,
} from "@/components/safety-findings";
import {
  SafetyWarnings,
  safetyGroupCount,
} from "@/components/safety-findings-affected";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { groupWarnings, partitionSafety } from "@/lib/group-findings";
import {
  driftDetail,
  kindLabel,
  STATE_BADGES,
  STATE_LABELS,
  safetyGroupCountLabel,
  scopeName,
  scopePath,
  toolName,
  UNMANAGED_SECTION_EXPLAINER,
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
  const changes = view.drift.filter((row) => row.state !== "unmanaged");
  const unmanaged = view.drift.filter((row) => row.state === "unmanaged");
  const orphans = view.drift.filter((row) => row.state === "orphaned");
  const { blocked, warn, clean } = partitionSafety(view.safety);
  const warnGroupCount = safetyGroupCount(warn);
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
      {/* Sections read top to bottom in order of urgency: held-back items
          can't be worked around, so they lead; changes are what applying
          this card does; safety warnings install anyway but deserve a
          look; not-managed items are pure housekeeping; an all-clear
          summary, if that's all there is, is the quietest thing here. */}
      <CardContent className="space-y-4">
        <BlockedFindings rows={blocked} />
        {changes.length > 0 ? (
          <div className="space-y-1.5">
            <SectionLabel>Changes</SectionLabel>
            <div className="divide-y divide-border">
              {changes.map((row) => {
                const detail = driftDetail(row);
                return (
                  <div
                    key={`${row.kind}:${row.name}:${row.harness}`}
                    className="flex flex-wrap items-center gap-2 py-2.5 first:pt-0 last:pb-0"
                  >
                    <span className="text-sm font-medium">{row.name}</span>
                    <Badge variant={STATE_BADGES[row.state]}>
                      {STATE_LABELS[row.state]}
                    </Badge>
                    <span className="text-xs text-muted-foreground">
                      {kindLabel(row.kind)} · {toolName(row.harness)}
                    </span>
                    {detail ? (
                      <span className="text-xs text-muted-foreground">
                        {detail}
                      </span>
                    ) : null}
                  </div>
                );
              })}
            </div>
          </div>
        ) : null}
        {view.notes.map((note) => (
          <p key={note} className="text-xs text-muted-foreground">
            {note}
          </p>
        ))}
        {groupWarnings(view.warnings).map((group) => (
          <p
            key={`${group.message}-${group.remediation ?? ""}`}
            className="text-xs text-muted-foreground"
          >
            <span className="break-all font-mono">
              {group.items.map((item) => item.name).join(", ")}
            </span>
            : {group.message}
            {group.remediation ? ` — fix: ${group.remediation}` : ""}
          </p>
        ))}
        {warn.length > 0 || clean.length > 0 ? (
          <div className="space-y-1.5">
            <SectionLabel>
              Safety
              {warnGroupCount > 0 ? (
                <span className="ml-1.5 font-normal normal-case tracking-normal text-muted-foreground">
                  · {safetyGroupCountLabel(warnGroupCount)}
                </span>
              ) : null}
            </SectionLabel>
            <SafetyWarnings rows={warn} />
            <SafetyCleanSummary rows={clean} />
          </div>
        ) : null}
        {unmanaged.length > 0 ? (
          <div className="space-y-1.5">
            <div className="flex items-baseline justify-between gap-2">
              <SectionLabel>Not managed yet</SectionLabel>
              <span className="text-xs text-muted-foreground">
                {unmanaged.length}
              </span>
            </div>
            <p className="text-xs text-muted-foreground">
              {UNMANAGED_SECTION_EXPLAINER}
            </p>
            <div className="divide-y divide-border/60 rounded-lg border bg-muted/30">
              {unmanaged.map((row) => (
                <div
                  key={`${row.kind}:${row.name}:${row.harness}`}
                  className="flex items-center gap-2 px-3 py-2.5"
                >
                  <span className="min-w-0 flex-1">
                    <span className="text-sm font-medium">{row.name}</span>{" "}
                    <span className="text-xs text-muted-foreground">
                      {kindLabel(row.kind)} · {toolName(row.harness)}
                    </span>
                    <span className="block truncate font-mono text-xs text-muted-foreground">
                      {row.detail}
                    </span>
                  </span>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="shrink-0"
                    disabled={busy}
                    onClick={() => onAdopt(row.kind, row.name, row.harness)}
                  >
                    Start managing
                  </Button>
                </div>
              ))}
            </div>
          </div>
        ) : null}
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
