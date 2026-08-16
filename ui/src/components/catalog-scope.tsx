import { GitBranch } from "lucide-react";
import { useState } from "react";
import type { Scope, SourceRow } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Section } from "@/components/section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { scopeName, scopePath } from "@/lib/labels";

export function CatalogScopeGroup({
  scope,
  rows,
  busy,
  onToggle,
  onRemove,
}: {
  scope: Scope;
  rows: SourceRow[];
  busy: boolean;
  onToggle: (name: string, enabled: boolean) => void;
  onRemove: (name: string) => void;
}) {
  // A scope with no catalogs used to get a card, a heading and an "Add a
  // catalog" button of its own — three of those stacked under one that
  // already said the same thing. An empty scope has nothing to show, so it
  // shows nothing; the section above says it once.
  if (rows.length === 0) return null;
  const path = scopePath(scope);
  return (
    <Section title={scopeName(scope)} description={path ?? undefined}>
      <div className="flex flex-col gap-3">
        {rows.map((row) => (
          <CatalogRow
            key={row.name}
            row={row}
            busy={busy}
            onToggle={(enabled) => onToggle(row.name, enabled)}
            onRemove={() => onRemove(row.name)}
          />
        ))}
      </div>
    </Section>
  );
}

function CatalogRow({
  row,
  busy,
  onToggle,
  onRemove,
}: {
  row: SourceRow;
  busy: boolean;
  onToggle: (enabled: boolean) => void;
  onRemove: () => void;
}) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const itemCount = row.declaredItems.length;
  const stillInUse = itemCount > 0;

  return (
    <div
      className={`flex items-start justify-between gap-3 text-sm ${row.enabled ? "" : "opacity-60"}`}
    >
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-1">
        <GitBranch className="size-4 shrink-0 text-muted-foreground" />
        <span className="font-semibold">{row.name}</span>
        {/* Reference is a git ref (branch, tag, or a commit id someone
            pinned) — data, not a description, so it reads as a mono badge
            rather than the same prose line an item's description gets. */}
        <Badge variant="outline" className="break-all font-mono">
          {row.reference}
        </Badge>
        {row.head ? (
          <Badge variant="outline" className="font-mono">
            @{row.head}
          </Badge>
        ) : null}
        {row.enabled ? null : <Badge variant="secondary">Off</Badge>}
        <span className="text-muted-foreground">
          {stillInUse
            ? `${itemCount} item${itemCount === 1 ? "" : "s"} installed from here`
            : "Nothing installed from here yet"}
        </span>
      </div>
      <span className="flex shrink-0 items-center gap-2">
        <Switch
          aria-label={`Turn ${row.name} on or off`}
          checked={row.enabled}
          disabled={busy}
          onCheckedChange={(checked) => onToggle(checked)}
        />
        <Button
          variant="outline"
          size="sm"
          disabled={busy || stillInUse}
          title={
            stillInUse
              ? `Still providing ${itemCount} items — turn it off instead`
              : undefined
          }
          onClick={() => setConfirmOpen(true)}
        >
          Remove…
        </Button>
      </span>
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={`Remove ${row.name}?`}
        description="Items already installed from it stay, but they'll no longer get updates."
        confirmLabel="Remove"
        destructive
        busy={busy}
        onConfirm={() => {
          onRemove();
          setConfirmOpen(false);
        }}
      />
    </div>
  );
}
