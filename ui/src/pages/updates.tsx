import {
  ChevronDown,
  ChevronRight,
  MoreHorizontal,
  RefreshCw,
} from "lucide-react";
import { useEffect, useState } from "react";
import type { UpdateRow } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { PageHeader } from "@/components/page-header";
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
  CHECK_FOR_UPDATES_LABEL,
  EDITED_UPDATE_TAG,
  hiddenUpdatesLabel,
  IGNORE_CONFIRM_BODY,
  IGNORE_CONFIRM_LABEL,
  IGNORE_UPDATES_LABEL,
  ignoreConfirmTitle,
  NOTIFY_AGAIN_LABEL,
  PINNED_UPDATE_TAG,
  PREVIEW_CHANGES_LABEL,
  UPDATE_ALL_LABEL,
  UPDATE_LABEL,
  UPDATES_EMPTY,
  UPDATES_SUBTITLE,
} from "@/lib/copy";
import { kindIcon } from "@/lib/kind-icon";
import { packageDisplayName } from "@/lib/labels";
import { CONTENT_WIDTH, PAGE_GUTTER } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { versionLabel } from "@/lib/versions";
import { useNavStore } from "@/stores/nav";
import {
  hiddenUpdates,
  useUpdatesStore,
  visibleUpdates,
} from "@/stores/updates";

function UpdateRowView({
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
            disabled={busy || row.blockedByLocalEdit}
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

/** Which packages have newer versions, what changed, and per-package
 *  control over how loudly to hear about it. */
export function UpdatesPage() {
  const { rows, busy, checking, check, updateAll } = useUpdatesStore();
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
