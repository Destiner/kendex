import { useEffect, useMemo, useRef } from "react";
import type { ItemKind, Tag } from "@/bindings";
import { InstalledRow } from "@/components/library/installed-row";
import { InstalledSkeleton } from "@/components/library/installed-skeleton";
import { LibraryFilters } from "@/components/library/library-filters";
import { LibraryLegend } from "@/components/library/library-legend";
import { NotManagedPanel } from "@/components/library/not-managed";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { TAGS_ROW_LABEL } from "@/lib/copy";
import { customizedItems } from "@/lib/customization";
import {
  filterItems,
  groupItems,
  groupScopes,
  projectScopes,
} from "@/lib/derive";
import { PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { scopeKey } from "@/lib/scope";
import { cn } from "@/lib/utils";
import { useEditorStore } from "@/stores/editor";
import { useLibraryViewStore } from "@/stores/library-view";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";

/** "Installed": everything on this machine, filterable. A row opens the
 *  package's own page; the filters and scroll position live in a store so
 *  coming back from that page lands exactly where the table was left. */
export function InstalledView() {
  const result = useScanStore((s) => s.result);
  const scope = useNavStore((s) => s.libraryScope);
  const setScope = useNavStore((s) => s.setLibraryScope);
  const goToLibrary = useNavStore((s) => s.goToLibrary);
  const goToPackage = useNavStore((s) => s.goToPackage);
  const clearLibraryFilter = useNavStore((s) => s.clearLibraryFilter);
  const {
    kind,
    harness,
    tag,
    setKind,
    setHarness,
    setTag,
    setScrollTop,
    clearFilters: clearViewFilters,
  } = useLibraryViewStore();
  // Kept in nav rather than here so leaving for a package page and coming
  // back lands on the same narrowed table.
  const search = useNavStore((s) => s.search);
  const setSearch = useNavStore((s) => s.setSearch);
  const projects = result ? projectScopes(result) : [];
  const scroller = useRef<HTMLDivElement | null>(null);
  // Every scope's manifest, so a row can say whether you have changed the
  // package wherever it is installed — not only in the scope last edited.
  const saved = useEditorStore((s) => s.saved);
  const loadAll = useEditorStore((s) => s.loadAll);
  useEffect(() => {
    void loadAll();
  }, [loadAll]);
  const customizedKeys = useMemo(() => {
    const keys = new Set<string>();
    for (const [where, draft] of Object.entries(saved)) {
      for (const item of customizedItems(draft)) {
        keys.add(`${where}|${item.kind}:${item.name}`);
      }
    }
    return keys;
  }, [saved]);
  const isCustomizedHere = (group: ReturnType<typeof groupItems>[number]) =>
    groupScopes(group).some((scope) =>
      customizedKeys.has(`${scopeKey(scope)}|${group.kind}:${group.name}`),
    );

  // The filter is a one-time handoff from wherever the link was clicked
  // (Harnesses, Projects); once applied, further tab visits start from the
  // stored view again rather than reapplying a stale filter.
  useEffect(() => {
    const handoff = useNavStore.getState().libraryFilter;
    if (handoff) {
      setKind(handoff.kind ?? "any");
      setHarness(handoff.harness ?? "any");
    }
    clearLibraryFilter();
  }, [clearLibraryFilter, setKind, setHarness]);

  // Restore where the table was scrolled to when it last unmounted, and
  // record it again on the way out.
  useEffect(() => {
    const node = scroller.current;
    if (!node) return;
    node.scrollTop = useLibraryViewStore.getState().scrollTop;
    return () => setScrollTop(node.scrollTop);
  }, [setScrollTop]);

  const groups = useMemo(() => {
    if (!result) return [];
    const filtered = filterItems(result.items, {
      scope,
      kind: kind === "any" ? undefined : (kind as ItemKind),
      harness: harness === "any" ? undefined : harness,
      tag: tag === "any" ? undefined : (tag as Tag),
      search,
    });
    return groupItems(filtered);
  }, [result, scope, kind, harness, tag, search]);

  // The count the filtered total is measured against: every row the table
  // could show, not the ones left after the current narrowing.
  const total = useMemo(
    () => (result ? groupItems(result.items).length : 0),
    [result],
  );
  // Nothing has been counted yet — distinct from "counted, found nothing".
  const scanning = result === null;
  const hasAnyItems = (result?.items.length ?? 0) > 0;
  const filtered =
    search !== "" ||
    kind !== "any" ||
    harness !== "any" ||
    tag !== "any" ||
    scope !== "all";

  const clearFilters = () => {
    clearViewFilters();
    setSearch("");
    setScope("all");
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <LibraryFilters
        search={search}
        onSearchChange={setSearch}
        kind={kind}
        onKindChange={setKind}
        harness={harness}
        onHarnessChange={setHarness}
        tag={tag}
        onTagChange={setTag}
        scope={scope}
        onScopeChange={setScope}
        projects={projects}
        shown={groups.length}
        total={total}
        counting={scanning}
        filtered={filtered}
        onClear={clearFilters}
      />
      <div className={cn("flex min-h-0 flex-1 flex-col pt-6", PAGE_GUTTER)}>
        <div className={cn("flex min-h-0 flex-1", WIDE_CONTENT_WIDTH)}>
          <div
            ref={scroller}
            className="min-w-0 flex-1 overflow-y-auto pr-2 [scrollbar-gutter:stable]"
          >
            <NotManagedPanel />
            {groups.some(isCustomizedHere) ? <LibraryLegend /> : null}
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>{TAGS_ROW_LABEL}</TableHead>
                  <TableHead>Harnesses</TableHead>
                  <TableHead>Where</TableHead>
                  <TableHead className="text-right">Updated</TableHead>
                  <TableHead>Status</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {groups.map((group) => {
                  const primary = group.installations[0];
                  return (
                    <InstalledRow
                      key={group.key}
                      group={group}
                      customized={isCustomizedHere(group)}
                      onOpen={() => {
                        if (!primary) return;
                        goToPackage({
                          kind: group.kind,
                          name: group.name,
                          scope: primary.scope,
                        });
                      }}
                    />
                  );
                })}
                {scanning ? <InstalledSkeleton /> : null}
                {!scanning && groups.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={7} className="py-10">
                      {hasAnyItems ? (
                        <div className="flex flex-col items-center gap-3 text-center">
                          <div>
                            <p className="font-medium">Nothing matches</p>
                            <p className="text-sm text-muted-foreground">
                              Try a different search or filter.
                            </p>
                          </div>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={clearFilters}
                          >
                            Clear filters
                          </Button>
                        </div>
                      ) : (
                        <div className="flex flex-col items-center gap-3 text-center">
                          <div>
                            <p className="font-medium">Nothing installed yet</p>
                            <p className="text-sm text-muted-foreground">
                              Add a catalog to start installing skills and
                              agents.
                            </p>
                          </div>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => goToLibrary({ tab: "add" })}
                          >
                            Add from a catalog
                          </Button>
                        </div>
                      )}
                    </TableCell>
                  </TableRow>
                ) : null}
              </TableBody>
            </Table>
          </div>
        </div>
      </div>
    </div>
  );
}
