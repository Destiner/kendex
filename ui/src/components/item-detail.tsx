import { useState } from "react";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { ReportDialog } from "@/components/report-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { ItemGroup } from "@/lib/derive";
import { kindLabel, scopeName, scopePath, toolName } from "@/lib/labels";
import { cn } from "@/lib/utils";
import { useAuditStore } from "@/stores/audit";

export function ItemDetail({ group }: { group: ItemGroup }) {
  const { busy, toggle, removeItem } = useAuditStore();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const managed = group.kind === "agent" || group.kind === "skill";
  const anyDisabled = group.installations.some((i) => i.enabled === false);
  const scope = group.installations[0]?.scope;

  return (
    <aside className="w-96 shrink-0 overflow-y-auto border-l p-5">
      <h2 className="font-semibold">{group.name}</h2>
      <p className="mb-1 text-sm text-muted-foreground">
        {kindLabel(group.kind)}
      </p>
      {group.description ? (
        <p className="mb-4 text-sm text-muted-foreground">
          {group.description}
        </p>
      ) : null}
      {scope ? (
        <div className="mb-4 flex gap-2">
          {managed ? (
            <Button
              size="sm"
              variant="outline"
              disabled={busy}
              onClick={() => void toggle(scope, group.name, anyDisabled)}
            >
              {anyDisabled ? "Turn on" : "Turn off"}
            </Button>
          ) : null}
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => setConfirmOpen(true)}
          >
            Remove…
          </Button>
          <ReportDialog scope={scope} name={group.name} kind={group.kind} />
        </div>
      ) : null}
      <div className="space-y-4">
        {group.installations.map((install) => (
          <div
            key={`${install.harness}:${install.scope.scope === "global" ? "global" : install.scope.root}:${install.path}`}
            className="rounded-md border p-3 text-sm"
          >
            <div className="mb-1 flex flex-wrap items-center gap-2">
              <Badge variant="outline">{toolName(install.harness)}</Badge>
              <span className="text-xs text-muted-foreground">
                {scopeName(install.scope)}
              </span>
              {scopePath(install.scope) ? (
                <span className="font-mono text-[10px] text-muted-foreground">
                  {scopePath(install.scope)}
                </span>
              ) : null}
              {install.enabled === false ? (
                <Badge variant="secondary">Off</Badge>
              ) : null}
            </div>
            <p className="break-all font-mono text-xs text-muted-foreground">
              {install.path}
            </p>
            {install.fileState.state === "symlink" ? (
              <p
                className={cn(
                  "mt-1 break-all text-xs",
                  install.fileState.broken
                    ? "text-destructive"
                    : "text-muted-foreground",
                )}
              >
                {install.fileState.broken
                  ? "The link is broken"
                  : `Linked from ${install.fileState.target}`}
              </p>
            ) : null}
            {install.origin ? (
              <p className="mt-1 break-all text-xs text-muted-foreground">
                {install.origin === "local"
                  ? "Managed from this project"
                  : `From ${install.origin}`}
              </p>
            ) : null}
          </div>
        ))}
      </div>
      {scope ? (
        <ConfirmDialog
          open={confirmOpen}
          onOpenChange={setConfirmOpen}
          title={`Remove ${group.name}?`}
          description="The files vstack manages will be moved to the trash, and it will stop being kept up to date."
          confirmLabel="Remove"
          destructive
          busy={busy}
          onConfirm={() => {
            void removeItem(scope, group.name).then(() =>
              setConfirmOpen(false),
            );
          }}
        />
      ) : null}
    </aside>
  );
}
