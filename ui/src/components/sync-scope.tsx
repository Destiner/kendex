import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { AuditView, DismissReason, DriftRow } from "@/bindings";
import { ApplyDialog } from "@/components/apply-dialog";
import { SafetyCleanSummary } from "@/components/safety-findings";
import { SafetyWarnings } from "@/components/safety-findings-affected";
import { BlockedFindings } from "@/components/safety-findings-blocked";
import { ScopeChanges, ScopeNotes } from "@/components/scope-details";
import { Section } from "@/components/section";
import { Button } from "@/components/ui/button";
import { UnmanagedItems } from "@/components/unmanaged-items";
import {
  APPLY_BUTTON_LABEL,
  NOTHING_TO_DO_HERE,
  scopeSummaryLabel,
} from "@/lib/copy";
import { openDecisionsLabel } from "@/lib/copy-safety";
import { mergeDriftRows } from "@/lib/drift-merge";
import { partitionSafety } from "@/lib/group-findings";
import { mergeHeldBack } from "@/lib/group-findings-blocked";
import { scopeName, scopePath } from "@/lib/labels";
import { evidenceGroups, openOccurrences } from "@/lib/reviewable";

/**
 * One project (or Personal), as its own panel.
 *
 * A machine with six projects used to be six full pages stacked end to end
 * with nothing but whitespace between them. Each is a container of its own
 * now, headed by what it needs and the button that does it, and a project
 * with nothing urgent starts closed — the header still says what's inside,
 * so nothing is hidden, it just isn't all shouting at once.
 */
export function SyncScopeCard({
  view,
  busy,
  onApply,
  onAdopt,
  onDismiss,
}: {
  view: AuditView;
  busy: boolean;
  onApply: (removeOrphans: boolean, allowUnsafe?: string[]) => void;
  onAdopt: (
    kind: DriftRow["kind"],
    name: string,
    harness: DriftRow["harness"],
    opts?: { silent?: boolean },
  ) => void;
  onDismiss: (tokens: string[], reason: DismissReason) => void;
}) {
  const [applyOpen, setApplyOpen] = useState(false);
  const changes = mergeDriftRows(
    view.drift.filter((row) => row.state !== "unmanaged"),
  );
  const unmanaged = mergeDriftRows(
    view.drift.filter((row) => row.state === "unmanaged"),
  );
  const orphans = mergeDriftRows(
    view.drift.filter((row) => row.state === "orphaned"),
  );
  const {
    blocked,
    open: undecided,
    settled,
    clean,
  } = partitionSafety(view.safety);
  // The panel counts what it renders: on-disk blocked rows plus the
  // plan-time refusals that never reached disk (view.heldBack), and one
  // decision per distinct piece of open evidence.
  const blockedCount = mergeHeldBack(blocked, view.heldBack).display.length;
  const openCount = evidenceGroups(openOccurrences(undecided)).length;
  // With nothing else to fix, removing left-behind items is the only
  // change on offer — defaulting the checkbox on keeps it reachable.
  const orphansOnly = orphans.length > 0 && view.plan.length === 0;
  const [removeOrphans, setRemoveOrphans] = useState(orphansOnly);
  const canApply = view.plan.length > 0 || orphans.length > 0;
  const summary = scopeSummaryLabel({
    changes: changes.length,
    blocked: blockedCount,
    open: openCount,
    unmanaged: unmanaged.length,
  });
  const [open, setOpen] = useState(blockedCount > 0 || canApply);
  const path = scopePath(view.scope);

  return (
    <section className="overflow-hidden rounded-xl border bg-card">
      <div className="flex items-center gap-3 px-4 py-3">
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          className="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5 text-left"
        >
          {open ? (
            <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
          ) : (
            <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
          )}
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-sm font-semibold">
              {scopeName(view.scope)}
            </span>
            <span className="truncate text-[13px] text-muted-foreground">
              {summary ?? NOTHING_TO_DO_HERE}
            </span>
          </span>
        </button>
        {path ? (
          <span className="hidden min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground lg:block">
            {path}
          </span>
        ) : null}
        {canApply ? (
          <Button
            size="sm"
            className="shrink-0"
            disabled={busy}
            onClick={() => {
              if (orphansOnly) setRemoveOrphans(true);
              setApplyOpen(true);
            }}
          >
            {APPLY_BUTTON_LABEL}
          </Button>
        ) : null}
      </div>
      {/* Sections read top to bottom in order of urgency: serious findings
          can't be worked around, so they lead; changes are what applying
          does; safety warnings install anyway but deserve a look;
          not-managed items are pure housekeeping. */}
      {open ? (
        <div className="flex flex-col gap-6 border-t px-4 py-4">
          <BlockedFindings
            rows={blocked}
            heldBack={view.heldBack}
            busy={busy}
            projectScope={view.scope.scope === "project"}
            onAccept={(tokens) => onApply(false, tokens)}
          />
          <ScopeChanges changes={changes} />
          <ScopeNotes notes={view.notes} warnings={view.warnings} />
          {undecided.length > 0 || settled.length > 0 || clean.length > 0 ? (
            <Section
              title="Safety"
              description={
                openCount > 0 ? openDecisionsLabel(openCount) : undefined
              }
            >
              <SafetyWarnings
                rows={undecided}
                projectScope={view.scope.scope === "project"}
                busy={busy}
                onDismiss={onDismiss}
              />
              <SafetyCleanSummary rows={clean} settled={settled} />
            </Section>
          ) : null}
          <UnmanagedItems rows={unmanaged} busy={busy} onAdopt={onAdopt} />
        </div>
      ) : null}
      <ApplyDialog
        open={applyOpen}
        onOpenChange={setApplyOpen}
        view={view}
        orphans={orphans}
        busy={busy}
        removeOrphans={removeOrphans}
        onRemoveOrphansChange={setRemoveOrphans}
        onApply={() => {
          onApply(removeOrphans);
          setApplyOpen(false);
        }}
      />
    </section>
  );
}
