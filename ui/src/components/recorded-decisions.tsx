import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import type { DecisionsView, RecordedDecision } from "@/bindings";
import { commands } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Section, SettingRow } from "@/components/section";
import { StatusLine, StatusNote } from "@/components/status-note";
import { Button } from "@/components/ui/button";
import {
  DECISIONS_SECTION_EXPLAINER,
  DECISIONS_SECTION_TITLE,
  decisionsErrorTitle,
  NO_LONGER_INSTALLED,
  noLongerApplies,
  TAKEN_BACK_TOAST,
} from "@/lib/copy-decisions";
import {
  decisionDetail,
  describeDecision,
  revokeLabel,
  sortDecisions,
} from "@/lib/decisions";
import { harnessName, kindLabel, scopeName } from "@/lib/labels";
import { useAuditStore } from "@/stores/audit";
import { useProblemsStore } from "@/stores/problems";

// A hook keeps its full id here — event, matcher, script — because seven
// hooks in one settings file all shorten to the same word, and a list of
// seven identical rows each with its own Take back tells nobody anything.
function rowTitle(row: RecordedDecision): string {
  const name = row.name;
  const kind = row.kind ? kindLabel(row.kind) : null;
  const tool = row.harness ? harnessName(row.harness) : null;
  return [name, [kind, tool].filter(Boolean).join(" · ")]
    .filter(Boolean)
    .join(" — ");
}

/** What the row's button will do, said before it does it. */
function confirmCopy(row: RecordedDecision): { title: string; body: string } {
  const name = row.name;
  if (row.state.state !== "active") {
    return {
      title: `Forget this decision about ${name}?`,
      body: "It no longer applies to anything, so forgetting it changes nothing on disk — the record simply leaves the file.",
    };
  }
  if (row.record.kind === "accepted") {
    return {
      title: `Withdraw the acceptance of ${name}?`,
      body: "The item is held back again. The next apply moves vstack's installed copy to the trash.",
    };
  }
  return {
    title: `Take back this dismissal on ${name}?`,
    body: "The finding comes back and asks for a decision again. Nothing on disk changes.",
  };
}

/** Every recorded decision across every scope — acceptances and
 *  dismissals — with whether each still applies and its way out. */
export function RecordedDecisions() {
  const [view, setView] = useState<DecisionsView>({
    decisions: [],
    errors: [],
  });
  const [revoking, setRevoking] = useState<RecordedDecision | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    const response = await commands.listDecisions();
    if (response.status === "ok") setView(response.data);
  }, []);
  useEffect(() => {
    void load();
  }, [load]);

  const revoke = async (row: RecordedDecision) => {
    setBusy(true);
    try {
      const response =
        row.record.kind === "accepted"
          ? await commands.revokeSafetyOverride(row.scope, row.key)
          : await commands.revokeDismissal(
              row.scope,
              row.key,
              row.record.fingerprint,
              row.record.dismissedAt,
            );
      if (response.status === "ok") {
        toast.success(
          row.record.kind === "accepted"
            ? `${row.name} is held back again`
            : TAKEN_BACK_TOAST,
        );
        await load();
        await useAuditStore.getState().refresh({ force: true });
      } else {
        useProblemsStore.getState().showError({
          title: "Couldn't take this decision back",
          message: response.error,
        });
      }
    } finally {
      setBusy(false);
      setRevoking(null);
    }
  };

  const rows = sortDecisions(view.decisions);
  if (rows.length === 0 && view.errors.length === 0) return null;
  const now = Date.now();
  const confirm = revoking ? confirmCopy(revoking) : null;
  return (
    <Section
      title={DECISIONS_SECTION_TITLE}
      description={DECISIONS_SECTION_EXPLAINER}
    >
      {view.errors.map((failed) => (
        <StatusNote
          key={scopeName(failed.scope)}
          tone="critical"
          title={decisionsErrorTitle(scopeName(failed.scope))}
          className="mb-3"
        >
          {failed.error.message}
        </StatusNote>
      ))}
      {rows.map((row) => (
        <SettingRow
          key={`${scopeName(row.scope)}:${row.key}:${
            row.record.kind === "dismissed" ? row.record.fingerprint : ""
          }`}
          label={rowTitle(row)}
          description={
            <span className="flex flex-col gap-1">
              <span>{describeDecision(row, now)}</span>
              {decisionDetail(row) ? (
                <span className="text-muted-foreground">
                  “{decisionDetail(row)}”
                </span>
              ) : null}
              {row.state.state === "stale" ? (
                <StatusLine tone="warning">
                  {noLongerApplies(row.state.why)}
                </StatusLine>
              ) : null}
              {row.state.state === "obsolete" ? (
                <span>{NO_LONGER_INSTALLED}</span>
              ) : null}
            </span>
          }
        >
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => setRevoking(row)}
          >
            {revokeLabel(row)}
          </Button>
        </SettingRow>
      ))}
      <ConfirmDialog
        open={revoking != null}
        onOpenChange={(open) => {
          if (!open) setRevoking(null);
        }}
        title={confirm?.title ?? ""}
        description={confirm?.body ?? ""}
        confirmLabel={revoking ? revokeLabel(revoking) : ""}
        destructive={revoking?.record.kind === "accepted"}
        busy={busy}
        onConfirm={() => {
          if (revoking) void revoke(revoking);
        }}
      />
    </Section>
  );
}
