import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { DriftRow, ItemKind } from "@/bindings";
import { SectionHeading } from "@/components/section";
import { Button } from "@/components/ui/button";
import {
  HIDE_ITEMS_LABEL,
  START_MANAGING_LABEL,
  showAllItemsLabel,
  startManagingAllLabel,
  UNMANAGED_SECTION_EXPLAINER,
} from "@/lib/copy";
import type { MergedDriftRow } from "@/lib/drift-merge";
import { summarizePaths } from "@/lib/drift-merge";
import { kindLabel, toolName } from "@/lib/labels";

// A project can carry dozens of hand-made items nobody intends to triage one
// at a time. Past this many, the list folds behind a one-line summary so the
// section stays a footnote instead of swallowing the page.
const INLINE_LIMIT = 5;

function kindCounts(rows: MergedDriftRow[]): [ItemKind, number][] {
  const counts = new Map<ItemKind, number>();
  for (const row of rows) counts.set(row.kind, (counts.get(row.kind) ?? 0) + 1);
  return [...counts.entries()];
}

// A skill installed by hand for two harnesses is one thing to adopt, not
// two — so one row carries every tool badge and one button adopts every
// installation at once.
export function UnmanagedItems({
  rows,
  busy,
  onAdopt,
}: {
  rows: MergedDriftRow[];
  busy: boolean;
  onAdopt: (
    kind: DriftRow["kind"],
    name: string,
    harness: DriftRow["harness"],
    opts?: { silent?: boolean },
  ) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  if (rows.length === 0) return null;
  const foldable = rows.length > INLINE_LIMIT;
  const showList = !foldable || expanded;

  const adoptAll = (groups: MergedDriftRow[]) => {
    let index = 0;
    for (const group of groups) {
      for (const row of group.installations) {
        onAdopt(row.kind, row.name, row.harness, { silent: index > 0 });
        index += 1;
      }
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <SectionHeading>Not managed yet</SectionHeading>
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
              onClick={() => adoptAll(rows)}
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
                    onClick={() => adoptAll([group])}
                  >
                    {START_MANAGING_LABEL}
                  </Button>
                </div>
              );
            })
          : null}
      </div>
    </div>
  );
}
