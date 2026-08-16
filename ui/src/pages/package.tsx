import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { commands, type Scope, type VersionRow } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { DiffView } from "@/components/diff/diff-view";
import { FilePreview } from "@/components/package/file-preview";
import { PackageActions } from "@/components/package/package-actions";
import { PackageSidebar } from "@/components/package/package-sidebar";
import {
  type PackageView,
  usePackageData,
  usePackageDiff,
} from "@/components/package/use-package-data";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import {
  FORKED_BADGE_LABEL,
  updatedToastLabel,
  VERSION_ERROR_TITLE,
} from "@/lib/copy";
import { groupItems, groupScopes } from "@/lib/derive";
import { kindIcon } from "@/lib/kind-icon";
import { packageDisplayName } from "@/lib/labels";
import { PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { installedRow, latestRow, versionRowLabel } from "@/lib/versions";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useProblemsStore } from "@/stores/problems";
import { useScanStore } from "@/stores/scan";

/** One package, full page: identity and provenance on the left, the file
 *  being read (or the diff being considered) on the right. */
export function PackagePage() {
  const ref = useNavStore((s) => s.packageRef);
  const initialView = useNavStore((s) => s.packageView);
  const clearPackageView = useNavStore((s) => s.clearPackageView);
  const back = useNavStore((s) => s.back);
  const result = useScanStore((s) => s.result);
  const { busy, toggle, removeItem } = useAuditStore();
  const showError = useProblemsStore((s) => s.showError);

  const [view, setView] = useState<PackageView>(() =>
    initialView
      ? {
          mode: "diff",
          from: initialView.from,
          to: initialView.to,
          fromLabel: initialView.from.slice(0, 7),
          toLabel: initialView.to.slice(0, 7),
        }
      : { mode: "files", file: null },
  );
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [switching, setSwitching] = useState(false);

  useEffect(() => {
    if (initialView) clearPackageView();
  }, [initialView, clearPackageView]);

  const group = useMemo(() => {
    if (!ref || !result) return null;
    const matching = result.items.filter(
      (item) => item.kind === ref.kind && item.name === ref.name,
    );
    return groupItems(matching)[0] ?? null;
  }, [ref, result]);

  const { meta, files, versions, load } = usePackageData(ref);
  const diff = usePackageDiff(
    ref,
    view,
    group?.installations[0]?.harness ?? null,
  );

  // The scan no longer knows this package (removed, renamed): nothing to
  // show, so leave the way the user came.
  useEffect(() => {
    if (ref && result && !group) back();
  }, [ref, result, group, back]);

  if (!ref || !group) return null;
  const primary = group.installations[0];
  if (!primary) return null;

  const Icon = kindIcon(group.kind);
  const displayName = packageDisplayName(ref);
  const installed = installedRow(versions);
  const latest = latestRow(versions);
  const updateAvailable =
    latest != null && !latest.installed && installed != null;

  const inEveryScope = async (act: (scope: Scope) => Promise<void>) => {
    for (const scope of groupScopes(group)) await act(scope);
  };

  const switchTo = (row: VersionRow) => {
    setSwitching(true);
    void commands
      .packageSetRev(ref.scope, ref.kind, ref.name, row.id)
      .then((response) => {
        setSwitching(false);
        if (response.status === "error") {
          showError({ title: VERSION_ERROR_TITLE, message: response.error });
          return;
        }
        toast.success(
          updatedToastLabel(`${displayName} to ${versionRowLabel(row)}`),
        );
        load();
        void useScanStore.getState().refresh();
        void useAuditStore.getState().refresh({ force: true });
      });
  };

  const follow = () => {
    setSwitching(true);
    void commands
      .packageSetRev(ref.scope, ref.kind, ref.name, null)
      .then((response) => {
        setSwitching(false);
        if (response.status === "error") {
          showError({ title: VERSION_ERROR_TITLE, message: response.error });
          return;
        }
        load();
        void useScanStore.getState().refresh();
      });
  };

  const compare = (row: VersionRow) => {
    if (!installed) return;
    setView({
      mode: "diff",
      from: installed.id,
      to: row.id,
      fromLabel: versionRowLabel(installed),
      toLabel: versionRowLabel(row),
    });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <PageHeader
        wide
        title={
          <span className="flex items-center gap-2.5">
            <Icon className="size-5 shrink-0 text-muted-foreground" />
            <span className="min-w-0 truncate">{displayName}</span>
            {meta?.fork != null ? (
              <Badge variant="outline">{FORKED_BADGE_LABEL}</Badge>
            ) : null}
          </span>
        }
        subtitle={group.description ?? undefined}
        action={
          <PackageActions
            scope={primary.scope}
            kind={group.kind}
            name={group.name}
            primaryPath={primary.path}
            updateAvailable={updateAvailable}
            busy={busy || switching}
            onUpdate={() => latest && switchTo(latest)}
            onPreview={() => latest && compare(latest)}
            onRemove={() => setConfirmRemove(true)}
          />
        }
      />
      <div className={cn("min-h-0 flex-1 overflow-y-auto", PAGE_GUTTER)}>
        <div className={cn("pb-8", WIDE_CONTENT_WIDTH)}>
          {view.mode === "diff" ? (
            diff ? (
              <DiffView
                diff={diff}
                fromLabel={view.fromLabel}
                toLabel={view.toLabel}
                onClose={() => setView({ mode: "files", file: null })}
              />
            ) : (
              <p className="text-sm text-muted-foreground">Comparing…</p>
            )
          ) : (
            <div className="flex flex-col gap-8 lg:flex-row">
              <PackageSidebar
                group={group}
                primary={primary}
                meta={meta}
                versions={versions}
                files={files}
                selectedFile={view.file}
                busy={busy || switching}
                onToggle={(_, enable) =>
                  void inEveryScope((scope) =>
                    toggle(scope, group.kind, group.name, enable),
                  )
                }
                onSwitchVersion={switchTo}
                onCompare={compare}
                onFollow={follow}
                onSelectFile={(file) => setView({ mode: "files", file })}
              />
              <div className="min-w-0 flex-1">
                <FilePreview
                  scope={ref.scope}
                  kind={ref.kind}
                  name={ref.name}
                  path={view.file}
                />
              </div>
            </div>
          )}
        </div>
      </div>
      <ConfirmDialog
        open={confirmRemove}
        onOpenChange={setConfirmRemove}
        title={`Remove ${group.name}?`}
        description="The files vstack manages will be moved to the trash, and it will stop being kept up to date."
        confirmLabel="Remove"
        destructive
        busy={busy}
        onConfirm={() => {
          void inEveryScope((scope) =>
            removeItem(scope, group.kind, group.name),
          ).then(() => {
            setConfirmRemove(false);
            back();
          });
        }}
      />
    </div>
  );
}
