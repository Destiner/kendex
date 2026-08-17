import type { ReactNode } from "react";
import type {
  HarnessId,
  ObservedItem,
  PackageMeta_Serialize,
} from "@/bindings";
import { SectionHeading } from "@/components/section";
import { StatusLine } from "@/components/status-note";
import { TagBadges } from "@/components/tag-badge";
import { ToolBadge } from "@/components/tool-badge";
import { Badge } from "@/components/ui/badge";
import { TAGS_ROW_LABEL } from "@/lib/copy";
import type { ItemGroup } from "@/lib/derive";
import { kindLabel, scopeName } from "@/lib/labels";
import { relativeTime } from "@/lib/relative-time";
import { versionLabel } from "@/lib/versions";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex gap-3 text-sm">
      <dt className="w-20 shrink-0 text-muted-foreground">{label}</dt>
      <dd className="min-w-0 flex-1">{children}</dd>
    </div>
  );
}

/** The Details block of the package page: what the scan observed plus the
 *  provenance the manifest and catalog record. Rows render only when they
 *  have something to say. */
export function PackageMetaBlock({
  group,
  primary,
  meta,
}: {
  group: ItemGroup;
  primary: ObservedItem;
  meta: PackageMeta_Serialize | null;
}) {
  return (
    <div className="space-y-2.5">
      <SectionHeading>Details</SectionHeading>
      <dl className="space-y-2">
        <Row label="Type">{kindLabel(group.kind)}</Row>
        {group.tags.length > 0 ? (
          <Row label={TAGS_ROW_LABEL}>
            <TagBadges tags={group.tags} />
          </Row>
        ) : null}
        <Row label="Tools">
          <span className="flex flex-wrap gap-1">
            {group.harnesses.map((h) => (
              <ToolBadge key={h} harness={h as HarnessId} />
            ))}
            {group.shared ? (
              <Badge variant="secondary">Shared files</Badge>
            ) : null}
          </span>
        </Row>
        <Row label="Scope">{scopeName(primary.scope)}</Row>
        {meta?.current ? (
          <Row label="Version">{versionLabel(meta.current)}</Row>
        ) : null}
        {meta?.repo ? (
          <Row label="Source">
            <span className="break-all">{meta.repo}</span>
          </Row>
        ) : primary.origin === "local" ? (
          <Row label="Source">Managed from this machine</Row>
        ) : null}
        {meta?.catalog?.author ? (
          <Row label="Author">{meta.catalog.author}</Row>
        ) : null}
        {meta?.catalog?.license ? (
          <Row label="License">{meta.catalog.license}</Row>
        ) : null}
        <Row label="Path">
          <span className="break-all font-mono text-xs">{primary.path}</span>
        </Row>
        {group.modifiedAt != null ? (
          <Row label="Updated">
            {relativeTime(group.modifiedAt * 1000, Date.now())}
          </Row>
        ) : null}
      </dl>
      {primary.fileState.state === "symlink" && primary.fileState.broken ? (
        <StatusLine tone="critical">The link is broken.</StatusLine>
      ) : null}
    </div>
  );
}
