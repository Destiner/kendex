import type { HarnessId } from "@/bindings";
import { StatusDot } from "@/components/status-dot";
import { Badge } from "@/components/ui/badge";
import { TableCell, TableRow } from "@/components/ui/table";
import { groupScopes, type ItemGroup } from "@/lib/derive";
import { kindIcon } from "@/lib/kind-icon";
import { hookDisplayName, kindLabel, scopeName, toolName } from "@/lib/labels";
import { relativeTime } from "@/lib/relative-time";
import { cn } from "@/lib/utils";

export function InstalledRow({
  group,
  selected,
  onSelect,
}: {
  group: ItemGroup;
  selected: boolean;
  onSelect: () => void;
}) {
  const Icon = kindIcon(group.kind);
  const displayName =
    group.kind === "hook" ? hookDisplayName(group.name) : group.name;
  const scopes = groupScopes(group);
  const whereLabel =
    scopes.length === 1 ? scopeName(scopes[0]) : `${scopes.length} locations`;
  const whereTitle = scopes
    .map((s) => (s.scope === "global" ? "Personal" : s.root))
    .join(", ");

  return (
    <TableRow
      onClick={onSelect}
      className={cn("cursor-pointer", selected && "bg-muted/60")}
    >
      <TableCell className="font-medium">
        <span className="flex items-center gap-2">
          <Icon className="size-4 shrink-0 text-muted-foreground" />
          <span className="min-w-0">
            <span className="block truncate">{displayName}</span>
            {group.description ? (
              <span className="block max-w-96 truncate text-xs font-normal text-muted-foreground">
                {group.description}
              </span>
            ) : null}
          </span>
        </span>
      </TableCell>
      <TableCell className="text-muted-foreground">
        {kindLabel(group.kind)}
      </TableCell>
      <TableCell>
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
