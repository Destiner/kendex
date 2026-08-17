import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { DriftRow, HarnessId, ItemKind } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { SectionHeading } from "@/components/section";
import { Button } from "@/components/ui/button";
import {
  HIDE_ITEMS_LABEL,
  START_MANAGING_LABEL,
  showAllItemsLabel,
  startManagingAllLabel,
  UNMANAGED_SECTION_EXPLAINER,
} from "@/lib/copy";
import {
  ADOPT_SHARED_CONFIRM,
  ADOPT_SHARED_TITLE,
  adoptSharedBody,
} from "@/lib/copy-safety";
import type { MergedDriftRow } from "@/lib/drift-merge";
import { summarizePaths } from "@/lib/drift-merge";
import { kindLabel, toolName } from "@/lib/labels";
import { sameScope } from "@/stores/audit";
import { useScanStore } from "@/stores/scan";

// A project can carry dozens of hand-made items nobody intends to triage one
// at a time. Past this many, the list folds behind a one-line summary so the
// section stays a footnote instead of swallowing the page.
const INLINE_LIMIT = 5;

function kindCounts(rows: MergedDriftRow[]): [ItemKind, number][] {
  const counts = new Map<ItemKind, number>();
  for (const row of rows) counts.set(row.kind, (counts.get(row.kind) ?? 0) + 1);
  return [...counts.entries()];
}

interface SharedLink {
  group: MergedDriftRow;
  /** The harness whose link to adopt through — the core resolves the
   *  target and takes every sibling link with it in one plan. */
  harness: HarnessId;
  /** The real folder the links resolve to. */
  target: string;
  /** Every tool whose install is a link at that folder. */
  tools: string[];
}

// An install that is a live symlink adopts the *target* — a folder the
// user may have pointed several tools at. That is a bigger move than
// adopting a plain folder (the old folder is trashed, and links vstack
// cannot see will break), so it gets a confirmation naming the folder and
// every tool reading it. Detection reads the scan, which resolves links.
function sharedLinkOf(group: MergedDriftRow): SharedLink | null {
  const items = useScanStore.getState().result?.items ?? [];
  for (const row of group.installations) {
    const item = items.find(
      (it) =>
        it.kind === group.kind &&
        it.name === group.name &&
        it.harness === row.harness &&
        sameScope(it.scope, row.scope),
    );
    if (item?.fileState.state !== "symlink" || item.fileState.broken) {
      continue;
    }
    const target = item.fileState.target;
    const tools = items
      .filter(
        (it) =>
          it.kind === group.kind &&
          it.name === group.name &&
          sameScope(it.scope, row.scope) &&
          it.fileState.state === "symlink" &&
          !it.fileState.broken &&
          it.fileState.target === target,
      )
      .map((it) => toolName(it.harness));
    return {
      group,
      harness: row.harness,
      target,
      tools: [...new Set(tools)],
    };
  }
  return null;
}

// A skill installed by hand for two harnesses is one thing to adopt, not
// two — so one row carries every tool badge and one button adopts every
// installation at once.
export function UnmanagedItems({
  rows,
  busy,
  title = "Not managed yet",
  onAdopt,
}: {
  rows: MergedDriftRow[];
  busy: boolean;
  /** The heading, named per project where several are listed together. */
  title?: string;
  onAdopt: (
    kind: DriftRow["kind"],
    name: string,
    harness: DriftRow["harness"],
    opts?: { silent?: boolean },
  ) => void | Promise<void>;
}) {
  const [expanded, setExpanded] = useState(false);
  const [confirmingShared, setConfirmingShared] = useState<SharedLink | null>(
    null,
  );
  if (rows.length === 0) return null;
  const foldable = rows.length > INLINE_LIMIT;
  const showList = !foldable || expanded;

  // One adoption at a time: every apply takes the scope's writer lock, so
  // firing them together turns all but the first into "scope is busy".
  const adoptAll = async (groups: MergedDriftRow[]) => {
    let index = 0;
    let shared: SharedLink | null = null;
    for (const group of groups) {
      const link = sharedLinkOf(group);
      if (link) {
        // A shared folder needs its own confirmation; the first one found
        // opens it after the plain adoptions finish.
        shared ??= link;
        continue;
      }
      for (const row of group.installations) {
        await onAdopt(row.kind, row.name, row.harness, { silent: index > 0 });
        index += 1;
      }
    }
    if (shared) setConfirmingShared(shared);
  };

  return (
    <div className="flex flex-col gap-2">
      <SectionHeading>{title}</SectionHeading>
      <p className="text-[13px] text-muted-foreground">
        {UNMANAGED_SECTION_EXPLAINER}
      </p>
      <div className="divide-y divide-border/60 rounded-lg border bg-muted/30">
        {foldable ? (
          <div className="flex flex-wrap items-center gap-2 px-3 py-2.5">
            <button
              type="button"
              onClick={() => setExpanded((e) => !e)}
              className="flex min-w-0 flex-1 cursor-pointer items-center gap-1.5 text-left"
            >
              {expanded ? (
                <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
              ) : (
                <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
              )}
              <span className="truncate text-sm">
                {kindCounts(rows)
                  .map(([kind, count]) => `${count} ${kindLabel(kind, count)}`)
                  .join(" · ")}
              </span>
              <span className="shrink-0 text-xs text-muted-foreground">
                {expanded ? HIDE_ITEMS_LABEL : showAllItemsLabel(rows.length)}
              </span>
            </button>
            <Button
              size="sm"
              variant="outline"
              className="shrink-0"
              disabled={busy}
              onClick={() => void adoptAll(rows)}
            >
              {startManagingAllLabel(rows.length)}
            </Button>
          </div>
        ) : null}
        {showList
          ? rows.map((group) => {
              const paths = summarizePaths(
                group.installations.map((row) => row.detail),
              );
              const tools = group.installations
                .map((row) => toolName(row.harness))
                .join(", ");
              return (
                <div
                  key={`${group.kind}:${group.name}:${group.state}`}
                  className="flex items-center gap-2 px-3 py-2.5"
                >
                  <span className="min-w-0 flex-1">
                    <span className="text-sm font-medium">{group.name}</span>{" "}
                    <span className="text-xs text-muted-foreground">
                      {kindLabel(group.kind)} · {tools}
                    </span>
                    {paths ? (
                      <span
                        className="block truncate font-mono text-xs text-muted-foreground"
                        title={paths.title}
                      >
                        {paths.text}
                      </span>
                    ) : null}
                  </span>
                  <Button
                    size="sm"
                    variant="outline"
                    className="shrink-0"
                    disabled={busy}
                    onClick={() => void adoptAll([group])}
                  >
                    {START_MANAGING_LABEL}
                  </Button>
                </div>
              );
            })
          : null}
      </div>
      <ConfirmDialog
        open={confirmingShared != null}
        onOpenChange={(open) => {
          if (!open) setConfirmingShared(null);
        }}
        title={ADOPT_SHARED_TITLE}
        description={
          confirmingShared
            ? adoptSharedBody(confirmingShared.target, confirmingShared.tools)
            : undefined
        }
        confirmLabel={ADOPT_SHARED_CONFIRM}
        destructive
        busy={busy}
        onConfirm={() => {
          if (confirmingShared) {
            void onAdopt(
              confirmingShared.group.kind,
              confirmingShared.group.name,
              confirmingShared.harness,
            );
          }
          setConfirmingShared(null);
        }}
      />
    </div>
  );
}
