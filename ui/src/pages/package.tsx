import { useEffect, useMemo, useState } from "react";
import type { Scope, VersionRow } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { DiffView } from "@/components/diff/diff-view";
import { FilePreview } from "@/components/package/file-preview";
import { EditedNotice } from "@/components/package/fork-notice";
import { PackageActions } from "@/components/package/package-actions";
import { PackageHeader } from "@/components/package/package-header";
import { PackageSidebar } from "@/components/package/package-sidebar";
import {
  type PackageView,
  packageVersionActions,
  usePackageData,
  usePackageDiff,
} from "@/components/package/use-package-data";
import { groupItems, groupScopes } from "@/lib/derive";
import { packageDisplayName } from "@/lib/labels";
import { PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { sameScope } from "@/lib/scope";
import { cn } from "@/lib/utils";
import { installedRow, latestRow, versionRowLabel } from "@/lib/versions";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";
import { useUpdatesStore } from "@/stores/updates";

/** One package, full page: provenance left, the file or diff right. */
export function PackagePage() {
  const ref = useNavStore((s) => s.packageRef);
  const initialView = useNavStore((s) => s.packageView);
  const clearPackageView = useNavStore((s) => s.clearPackageView);
  const back = useNavStore((s) => s.back);
  const result = useScanStore((s) => s.result);
  const { busy, toggle, removeItem } = useAuditStore();

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
  const updatesLoaded = useUpdatesStore((s) => s.loaded);
  const edited = useUpdatesStore((s) =>
    s.rows.some(
      (row) =>
        ref != null &&
        row.kind === ref.kind &&
        row.name === ref.name &&
        sameScope(row.scope, ref.scope) &&
        row.blockedByLocalEdit,
    ),
  );

  // The scan no longer knows this package (removed, renamed): leave the
  // way the user came.
  useEffect(() => {
    if (ref && result && !group) back();
  }, [ref, result, group, back]);

  if (!ref || !group) return null;
  const primary = group.installations[0];
  if (!primary) return null;

  const displayName = packageDisplayName(ref);
  const installed = installedRow(versions);
  const latest = latestRow(versions);
  // Update waits for meta (held vs following) and the updates store
  // (edited), and is off while edits are held.
  const canUpdate =
    latest != null &&
    !latest.installed &&
    installed != null &&
    meta != null &&
    updatesLoaded &&
    !edited;

  const inEveryScope = async (act: (scope: Scope) => Promise<void>) => {
    for (const scope of groupScopes(group)) await act(scope);
  };

  const { switchTo, updateToLatest, follow } = packageVersionActions(
    ref,
    displayName,
    meta?.rev != null,
    setSwitching,
    load,
  );

  const compare = (row: VersionRow) =>
    installed &&
    setView({
      mode: "diff",
      from: installed.id,
      to: row.id,
      fromLabel: versionRowLabel(installed),
      toLabel: versionRowLabel(row),
    });

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <PackageHeader
        kind={group.kind}
        displayName={displayName}
        description={group.description}
        forked={meta?.fork != null}
        action={
          <PackageActions
            scope={primary.scope}
            kind={group.kind}
            name={group.name}
            primaryPath={primary.path}
            updateAvailable={canUpdate}
            busy={busy || switching}
            onUpdate={() => latest && updateToLatest(latest)}
            onPreview={() => latest && compare(latest)}
            onRemove={() => setConfirmRemove(true)}
          />
        }
      />
      <div className={cn("min-h-0 flex-1 overflow-y-auto", PAGE_GUTTER)}>
        <div className={cn("pb-8", WIDE_CONTENT_WIDTH)}>
          <EditedNotice
            scope={ref.scope}
            kind={ref.kind}
            name={ref.name}
            harness={primary.harness}
            alreadyForked={meta?.fork != null}
            onViewChanges={() => {
              if (!installed) return;
              setView({
                mode: "diff",
                from: installed.id,
                to: "installed",
                fromLabel: versionRowLabel(installed),
                toLabel: "your edits",
              });
            }}
            onResolved={load}
          />
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
            // A failed removal shows its error and leaves the page up —
            // the vanish-effect takes us back only once the package is
            // actually gone from the scan.
            if (
              !useScanStore
                .getState()
                .result?.items.some(
                  (item) =>
                    item.kind === group.kind && item.name === group.name,
                )
            ) {
              back();
            }
          });
        }}
      />
    </div>
  );
}
