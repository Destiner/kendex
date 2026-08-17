import { ChevronDown, ChevronRight, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { UpdateRow } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import { UpdateRowView } from "@/components/update-row";
import {
  CHECK_FOR_UPDATES_LABEL,
  hiddenUpdatesLabel,
  IGNORE_CONFIRM_BODY,
  IGNORE_CONFIRM_LABEL,
  ignoreConfirmTitle,
  UPDATE_ALL_LABEL,
  UPDATES_EMPTY,
  UPDATES_SUBTITLE,
  UPDATES_UNCHECKED_TITLE,
} from "@/lib/copy";
import { CONTENT_WIDTH, PAGE_GUTTER } from "@/lib/layout";
import { cn } from "@/lib/utils";
import {
  hiddenUpdates,
  useUpdatesStore,
  visibleUpdates,
} from "@/stores/updates";

/** Which packages have newer versions, what changed, and per-package
 *  control over how loudly to hear about it. */
export function UpdatesPage() {
  const { rows, warnings, busy, checking, check, updateAll } =
    useUpdatesStore();
  const load = useUpdatesStore((s) => s.load);
  const [showHidden, setShowHidden] = useState(false);
  const [confirmIgnore, setConfirmIgnore] = useState<UpdateRow | null>(null);

  useEffect(() => {
    void load();
  }, [load]);

  const visible = visibleUpdates(rows);
  const hidden = hiddenUpdates(rows);
  const HiddenChevron = showHidden ? ChevronDown : ChevronRight;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <PageHeader
        title="Updates"
        subtitle={UPDATES_SUBTITLE}
        action={
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              disabled={checking}
              onClick={() => void check()}
            >
              <RefreshCw
                className={cn("size-3.5", checking && "animate-spin")}
              />
              {CHECK_FOR_UPDATES_LABEL}
            </Button>
            {visible.length > 1 ? (
              <Button
                size="sm"
                disabled={busy}
                onClick={() => void updateAll()}
              >
                {UPDATE_ALL_LABEL}
              </Button>
            ) : null}
          </div>
        }
      />
      <div className={cn("min-h-0 flex-1 overflow-y-auto", PAGE_GUTTER)}>
        <div className={cn("pb-8", CONTENT_WIDTH)}>
          {visible.length === 0 ? (
            <p className="py-6 text-sm text-muted-foreground">
              {UPDATES_EMPTY}
            </p>
          ) : (
            <div className="divide-y">
              {visible.map((row) => (
                <UpdateRowView
                  key={`${row.kind}:${row.name}:${JSON.stringify(row.scope)}`}
                  row={row}
                  onIgnore={setConfirmIgnore}
                />
              ))}
            </div>
          )}
          {warnings.length > 0 ? (
            <div className="mt-8">
              <p className="text-sm font-medium">{UPDATES_UNCHECKED_TITLE}</p>
              <div className="mt-1 space-y-1">
                {warnings.map((warning) => (
                  <p
                    key={`${warning.kind}:${warning.name}:${warning.message}`}
                    className="text-xs text-muted-foreground"
                  >
                    {warning.name}: {warning.message}
                    {warning.remediation ? ` — ${warning.remediation}` : ""}
                  </p>
                ))}
              </div>
            </div>
          ) : null}
          {hidden.length > 0 ? (
            <div className="mt-8">
              <button
                type="button"
                className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
                onClick={() => setShowHidden((value) => !value)}
              >
                <HiddenChevron className="size-3.5" />
                {hiddenUpdatesLabel(hidden.length)}
              </button>
              {showHidden ? (
                <div className="mt-2 divide-y opacity-80">
                  {hidden.map((row) => (
                    <UpdateRowView
                      key={`${row.kind}:${row.name}:${JSON.stringify(row.scope)}`}
                      row={row}
                    />
                  ))}
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
      <ConfirmDialog
        open={confirmIgnore != null}
        onOpenChange={(open) => {
          if (!open) setConfirmIgnore(null);
        }}
        title={confirmIgnore ? ignoreConfirmTitle(confirmIgnore.name) : ""}
        description={IGNORE_CONFIRM_BODY}
        confirmLabel={IGNORE_CONFIRM_LABEL}
        busy={busy}
        onConfirm={() => {
          if (!confirmIgnore) return;
          void useUpdatesStore
            .getState()
            .setIgnored(confirmIgnore, true)
            .then(() => setConfirmIgnore(null));
        }}
      />
    </div>
  );
}
