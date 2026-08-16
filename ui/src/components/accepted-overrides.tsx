import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import type { AcceptedOverride } from "@/bindings";
import { commands } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Section, SettingRow } from "@/components/section";
import { Button } from "@/components/ui/button";
import {
  ACCEPTED_SECTION_EXPLAINER,
  ACCEPTED_SECTION_TITLE,
  acceptedFindingsCountLabel,
  WITHDRAW_LABEL,
} from "@/lib/copy-safety";
import { kindLabel, scopeName, toolName } from "@/lib/labels";
import { relativeTime } from "@/lib/relative-time";
import { useAuditStore } from "@/stores/audit";
import { useProblemsStore } from "@/stores/problems";

function rowTitle(row: AcceptedOverride): string {
  const kind = row.kind ? kindLabel(row.kind) : null;
  const tool = row.harness ? toolName(row.harness) : null;
  return [row.name, [kind, tool].filter(Boolean).join(" · ")]
    .filter(Boolean)
    .join(" — ");
}

function rowDetail(row: AcceptedOverride): string {
  const granted = Date.parse(row.grantedAt);
  const when = Number.isNaN(granted)
    ? null
    : `accepted ${relativeTime(granted, Date.now())}`;
  return [scopeName(row.scope), acceptedFindingsCountLabel(row.findings), when]
    .filter(Boolean)
    .join(" · ");
}

/** The recorded acceptances across every scope, each with its way out. */
export function AcceptedOverrides() {
  const [rows, setRows] = useState<AcceptedOverride[]>([]);
  const [withdrawing, setWithdrawing] = useState<AcceptedOverride | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    const response = await commands.listSafetyOverrides();
    if (response.status === "ok") setRows(response.data);
  }, []);
  useEffect(() => {
    void load();
  }, [load]);

  const withdraw = async (row: AcceptedOverride) => {
    setBusy(true);
    try {
      const response = await commands.revokeSafetyOverride(row.scope, row.key);
      if (response.status === "ok") {
        toast.success(`${row.name} is held back again`);
        await load();
        await useAuditStore.getState().refresh({ force: true });
      } else {
        useProblemsStore.getState().showError({
          title: "Couldn't withdraw this acceptance",
          message: response.error,
        });
      }
    } finally {
      setBusy(false);
      setWithdrawing(null);
    }
  };

  if (rows.length === 0) return null;
  return (
    <Section
      title={ACCEPTED_SECTION_TITLE}
      description={ACCEPTED_SECTION_EXPLAINER}
    >
      {rows.map((row) => (
        <SettingRow
          key={`${scopeName(row.scope)}:${row.key}`}
          label={rowTitle(row)}
          description={rowDetail(row)}
        >
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => setWithdrawing(row)}
          >
            {WITHDRAW_LABEL}
          </Button>
        </SettingRow>
      ))}
      <ConfirmDialog
        open={withdrawing != null}
        onOpenChange={(open) => {
          if (!open) setWithdrawing(null);
        }}
        title={`Withdraw the acceptance of ${withdrawing?.name ?? ""}?`}
        description="The item is held back again. The next apply moves vstack's installed copy to the trash."
        confirmLabel={WITHDRAW_LABEL}
        destructive
        busy={busy}
        onConfirm={() => {
          if (withdrawing) void withdraw(withdrawing);
        }}
      />
    </Section>
  );
}
