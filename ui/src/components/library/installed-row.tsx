import type { HarnessId } from "@/bindings";
import { StatusDot } from "@/components/status-dot";
import { TagBadges } from "@/components/tag-badge";
import { ToolBadge } from "@/components/tool-badge";
import { Badge } from "@/components/ui/badge";
import { TableCell, TableRow } from "@/components/ui/table";
import { FORKED_BADGE_LABEL } from "@/lib/copy";
import { groupScopes, type ItemGroup } from "@/lib/derive";
import { kindIcon } from "@/lib/kind-icon";
import {
  describesItself,
  hookDisplayName,
  kindLabel,
  scopeName,
} from "@/lib/labels";
import { relativeTime } from "@/lib/relative-time";
import { cn } from "@/lib/utils";
import { useUpdatesStore } from "@/stores/updates";

export function InstalledRow({
  group,
  onOpen,
}: {
  group: ItemGroup;
  onOpen: () => void;
}) {
  const Icon = kindIcon(group.kind);
  const forked = useUpdatesStore((s) =>
    s.rows.some(
      (row) => row.kind === group.kind && row.name === group.name && row.forked,
    ),
  );
  const displayName =
    group.kind === "hook" ? hookDisplayName(group.name) : group.name;
  const scopes = groupScopes(group);
  const whereLabel =
    scopes.length === 1 ? scopeName(scopes[0]) : `${scopes.length} locations`;
  const whereTitle = scopes
    .map((s) => (s.scope === "global" ? "Personal" : s.root))
    .join(", ");

  return (
    <TableRow onClick={onOpen} className="cursor-pointer">
      {/* Cells are nowrap by default; the description is the one column that
          wants to wrap rather than run out of the row and get cut mid-word. */}
      <TableCell className="max-w-[22rem] font-medium whitespace-normal">
        <span className="flex items-start gap-2">
          <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          <span className="min-w-0">
            <span className="flex items-center gap-1.5">
              <span className="block truncate">{displayName}</span>
              {forked ? (
                <Badge variant="outline">{FORKED_BADGE_LABEL}</Badge>
              ) : null}
            </span>
            {group.description ? (
              <span
                className={cn(
                  "line-clamp-2 text-xs font-normal text-muted-foreground",
                  !describesItself(group.kind) && "font-mono text-[11px]",
                )}
              >
                {group.description}
              </span>
            ) : null}
          </span>
        </span>
      </TableCell>
      <TableCell className="align-top text-muted-foreground">
        {kindLabel(group.kind)}
      </TableCell>
      <TableCell className="align-top">
        <TagBadges tags={group.tags} />
      </TableCell>
      <TableCell>
        <span className="flex flex-wrap gap-1">
          {group.harnesses.map((h) => (
            <ToolBadge key={h} harness={h as HarnessId} />
          ))}
          {group.shared ? (
            <Badge variant="secondary">Shared files</Badge>
          ) : null}
        </span>
      </TableCell>
      <TableCell title={whereTitle} className="text-muted-foreground">
        {whereLabel}
      </TableCell>
      <TableCell className="text-right text-xs text-muted-foreground">
        {group.modifiedAt != null
          ? relativeTime(group.modifiedAt * 1000, Date.now())
          : "—"}
      </TableCell>
      <TableCell>
        {group.installations.some((i) => i.enabled === false) ? (
          <Badge variant="secondary">Off</Badge>
        ) : (
          <span className="flex items-center gap-1.5 text-xs text-good">
            <StatusDot tone="good" />
            Active
          </span>
        )}
      </TableCell>
    </TableRow>
  );
}
