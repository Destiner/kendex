import type { ReactNode } from "react";
import type { HarnessId, ObservedItem } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { Badge } from "@/components/ui/badge";
import type { ItemGroup } from "@/lib/derive";
import { kindLabel, scopeName, toolName } from "@/lib/labels";
import { relativeTime } from "@/lib/relative-time";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex gap-3 text-sm">
      <dt className="w-20 shrink-0 text-muted-foreground">{label}</dt>
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

export function ItemDetailMeta({
  group,
  primary,
}: {
  group: ItemGroup;
  primary: ObservedItem;
}) {
  const provenance = provenanceLabel(primary.origin);
  return (
    <div className="space-y-2.5">
      <SectionLabel>Details</SectionLabel>
      <dl className="space-y-2">
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
          <span className="break-all font-mono text-xs">{primary.path}</span>
        </Row>
        {primary.fileState.state === "symlink" && !primary.fileState.broken ? (
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
  );
}
