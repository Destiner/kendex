import { MoreHorizontal } from "lucide-react";
import type { UpdateRow } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Switch } from "@/components/ui/switch";
import {
  AUTO_UPDATE_LABEL,
  EDITED_UPDATE_TAG,
  IGNORE_UPDATES_LABEL,
  NOTIFY_AGAIN_LABEL,
  PINNED_UPDATE_TAG,
  PREVIEW_CHANGES_LABEL,
  REMOVED_UPSTREAM_TAG,
  UPDATE_LABEL,
} from "@/lib/copy";
import { kindIcon } from "@/lib/kind-icon";
import { packageDisplayName } from "@/lib/labels";
import { versionLabel } from "@/lib/versions";
import { useNavStore } from "@/stores/nav";
import { useUpdatesStore } from "@/stores/updates";

export function UpdateRowView({
  row,
  onIgnore,
}: {
  row: UpdateRow;
  onIgnore?: (row: UpdateRow) => void;
}) {
  const { busy, updateOne, setAutoUpdate, setIgnored } = useUpdatesStore();
  const goToPackage = useNavStore((s) => s.goToPackage);
  const Icon = kindIcon(row.kind);
  const name = packageDisplayName(row);

  const preview = () => {
    if (!row.current || !row.latest) return;
    goToPackage(
      { kind: row.kind, name: row.name, scope: row.scope },
      { mode: "diff", from: row.current.commit, to: row.latest.commit },
    );
  };

  return (
    <div className="flex items-center gap-3 py-2.5">
      <Icon className="size-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <p className="flex items-center gap-2 text-sm font-medium">
          <span className="truncate">{name}</span>
          {row.pinned ? (
            <Badge variant="outline">{PINNED_UPDATE_TAG}</Badge>
          ) : null}
          {row.blockedByLocalEdit ? (
            <Badge variant="outline">{EDITED_UPDATE_TAG}</Badge>
          ) : null}
          {row.removedUpstream ? (
            <Badge variant="outline">{REMOVED_UPSTREAM_TAG}</Badge>
          ) : null}
        </p>
        <p className="font-mono text-xs text-muted-foreground">
          {row.current ? versionLabel(row.current) : "?"} →{" "}
          {row.latest ? versionLabel(row.latest) : "?"}
        </p>
      </div>
      {row.ignored ? (
        <Button
          size="sm"
          variant="outline"
          onClick={() => void setIgnored(row, false)}
        >
          {NOTIFY_AGAIN_LABEL}
        </Button>
      ) : (
        <>
          <span className="flex items-center gap-2 text-xs text-muted-foreground">
            {AUTO_UPDATE_LABEL}
            <Switch
              aria-label={AUTO_UPDATE_LABEL}
              checked={!row.pinned}
              disabled={busy}
              onCheckedChange={(auto) => void setAutoUpdate(row, auto)}
            />
          </span>
          <Button size="sm" variant="ghost" onClick={preview}>
            {PREVIEW_CHANGES_LABEL}
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={busy || row.blockedByLocalEdit || !row.updateAvailable}
            onClick={() => void updateOne(row)}
          >
            {UPDATE_LABEL}
          </Button>
          {onIgnore ? (
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    size="icon-xs"
                    variant="ghost"
                    aria-label="More actions"
                  >
                    <MoreHorizontal className="size-4" />
                  </Button>
                }
              />
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => onIgnore(row)}>
                  {IGNORE_UPDATES_LABEL}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          ) : null}
        </>
      )}
    </div>
  );
}
