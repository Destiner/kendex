import { X } from "lucide-react";
import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { commands, type HarnessId } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { ItemPreview } from "@/components/library/item-preview";
import { ReportDialog } from "@/components/report-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { ItemGroup } from "@/lib/derive";
import { kindIcon } from "@/lib/kind-icon";
import { hookDisplayName, kindLabel, scopeName, toolName } from "@/lib/labels";
import { relativeTime } from "@/lib/relative-time";
import { useAuditStore } from "@/stores/audit";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex gap-3 text-sm">
      <dt className="w-16 shrink-0 text-muted-foreground">{label}</dt>
      <dd className="min-w-0 flex-1">{children}</dd>
    </div>
  );
}

// The engine only ever fills origin with "local" (this project's own
// manifest) or a catalog's repo slug — anything else means it has no
// provenance to report at all.
function provenanceLabel(origin: string | null): string | null {
  if (!origin) return null;
  return origin === "local" ? "Managed from this project" : `From ${origin}`;
}

export function ItemDetail({
  group,
  onClose,
}: {
  group: ItemGroup;
  onClose: () => void;
}) {
  const { busy, toggle, removeItem } = useAuditStore();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const managed = group.kind === "agent" || group.kind === "skill";
  const anyDisabled = group.installations.some((i) => i.enabled === false);

  // groupItems only ever creates a group from at least one observed
  // installation, so the first one stands in for the item as a whole.
  const primary = group.installations[0];

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  if (!primary) return null;

  const Icon = kindIcon(group.kind);
  const displayName =
    group.kind === "hook" ? hookDisplayName(group.name) : group.name;
  const provenance = provenanceLabel(primary.origin);

  const revealInFileBrowser = () => {
    void commands.revealPath(primary.path).then((response) => {
      if (response.status === "error") toast.error(response.error);
    });
  };

  return (
    <aside className="flex w-96 shrink-0 flex-col overflow-y-auto border-l bg-card">
      <div className="flex items-start justify-between gap-2 p-5 pb-3">
        <div className="flex min-w-0 items-center gap-2">
          <Icon className="size-5 shrink-0 text-muted-foreground" />
          <h2 className="truncate font-semibold">{displayName}</h2>
        </div>
        <Button
          variant="ghost"
          size="icon-xs"
          aria-label="Close"
          title="Close"
          onClick={onClose}
        >
          <X className="size-4" />
        </Button>
      </div>
      <div className="flex-1 space-y-5 px-5 pb-5">
        {group.description ? (
          <p className="text-sm text-muted-foreground">{group.description}</p>
        ) : null}

        <div className="space-y-2">
          <SectionLabel>Details</SectionLabel>
          <dl className="space-y-1.5">
            <Row label="Type">{kindLabel(group.kind)}</Row>
            <Row label="Tools">
              <span className="flex flex-wrap gap-1">
                {group.harnesses.map((h) => (
                  <Badge key={h} variant="outline">
                    {toolName(h as HarnessId)}
                  </Badge>
                ))}
                {group.shared ? (
                  <Badge variant="secondary">Shared files</Badge>
                ) : null}
              </span>
            </Row>
            <Row label="Scope">{scopeName(primary.scope)}</Row>
            <Row label="Path">
              <span className="break-all font-mono text-xs">
                {primary.path}
              </span>
            </Row>
            {primary.fileState.state === "symlink" &&
            !primary.fileState.broken ? (
              <Row label="Linked">
                <span className="break-all font-mono text-xs">
                  {primary.fileState.target}
                </span>
              </Row>
            ) : null}
            {group.modifiedAt != null ? (
              <Row label="Updated">
                {relativeTime(group.modifiedAt * 1000, Date.now())}
              </Row>
            ) : null}
            {provenance ? <Row label="Source">{provenance}</Row> : null}
          </dl>
          {primary.fileState.state === "symlink" && primary.fileState.broken ? (
            <p className="text-xs text-destructive">The link is broken.</p>
          ) : null}
        </div>

        <div className="flex flex-wrap gap-2">
          {managed ? (
            <Button
              size="sm"
              variant="outline"
              disabled={busy}
              onClick={() =>
                void toggle(primary.scope, group.name, anyDisabled)
              }
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
          <Button size="sm" variant="outline" onClick={revealInFileBrowser}>
            Show in file browser
          </Button>
          <ReportDialog
            scope={primary.scope}
            name={group.name}
            kind={group.kind}
          />
        </div>

        <div className="space-y-2">
          <SectionLabel>Content</SectionLabel>
          <ItemPreview
            scope={primary.scope}
            kind={group.kind}
            name={group.name}
            harness={primary.harness}
          />
        </div>
      </div>
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={`Remove ${group.name}?`}
        description="The files vstack manages will be moved to the trash, and it will stop being kept up to date."
        confirmLabel="Remove"
        destructive
        busy={busy}
        onConfirm={() => {
          void removeItem(primary.scope, group.name).then(() =>
            setConfirmOpen(false),
          );
        }}
      />
    </aside>
  );
}
