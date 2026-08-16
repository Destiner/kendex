import { useEffect, useMemo, useState } from "react";
import type { ItemKind, Tag } from "@/bindings";
import { ItemDetail } from "@/components/item-detail";
import { InstalledRow } from "@/components/library/installed-row";
import { LibraryFilters } from "@/components/library/library-filters";
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
import {
  filterItems,
  groupItems,
  type Location,
  projectScopes,
} from "@/lib/derive";
import { PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";

/** "Installed": everything on this machine, filterable, with a detail pane. */
export function InstalledView() {
  const result = useScanStore((s) => s.result);
  const scope = useNavStore((s) => s.scope);
  const goToLibrary = useNavStore((s) => s.goToLibrary);
  const clearLibraryFilter = useNavStore((s) => s.clearLibraryFilter);
  const [kind, setKind] = useState<string>(
    () => useNavStore.getState().libraryFilter?.kind ?? "any",
  );
  const [harness, setHarness] = useState<string>(
    () => useNavStore.getState().libraryFilter?.tool ?? "any",
  );
  // Page-local, alongside kind/harness — narrows within whatever the
  // sidebar's global scope already shows, rather than replacing it.
  // Empty set is "All".
  const [tag, setTag] = useState<string>("any");
  const [locations, setLocations] = useState<Set<Location>>(() => new Set());
  // The search box lives in the sidebar, one for the whole app.
  const search = useNavStore((s) => s.search);
  const setSearch = useNavStore((s) => s.setSearch);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const projects = result ? projectScopes(result) : [];

  // The filter is a one-time handoff from wherever the link was clicked
  // (Tools, Projects); once applied, further tab visits start from "any"
  // again rather than reapplying a stale filter.
  useEffect(() => {
    clearLibraryFilter();
  }, [clearLibraryFilter]);

  const groups = useMemo(() => {
    if (!result) return [];
    const filtered = filterItems(result.items, {
      scope,
      locations,
      kind: kind === "any" ? undefined : (kind as ItemKind),
      harness: harness === "any" ? undefined : harness,
      tag: tag === "any" ? undefined : (tag as Tag),
      search,
    });
    return groupItems(filtered);
  }, [result, scope, locations, kind, harness, tag, search]);

  const selected = groups.find((g) => g.key === selectedKey) ?? null;
  const hasAnyItems = (result?.items.length ?? 0) > 0;

  const clearFilters = () => {
    setKind("any");
    setHarness("any");
    setTag("any");
    setLocations(new Set());
    setSearch("");
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <LibraryFilters
        kind={kind}
        onKindChange={setKind}
        harness={harness}
        onHarnessChange={setHarness}
        tag={tag}
        onTagChange={setTag}
        locations={locations}
        onLocationsChange={setLocations}
        projects={projects}
      />
      <div className={cn("flex min-h-0 flex-1 flex-col", PAGE_GUTTER)}>
        {/* The flyout floats above this pane rather than sharing the row
            with it, so the table keeps its full width — and its columns
            stop truncating — whether or not a row is selected. Where the
            window is wide enough to spare it, the table shifts clear of the
            open panel instead of hiding under it. */}
        <div
          className={cn(
            "flex min-h-0 flex-1 transition-[padding] duration-200 ease-out",
            WIDE_CONTENT_WIDTH,
            selected && "xl:pr-[min(30rem,85vw)]",
          )}
        >
          {/* Frozen while the panel is open: the table is behind a
              click-catcher anyway, and an overlay scrollbar draws in a layer
              of its own that would otherwise linger on top of the panel
              until the compositor fades it out. Reserve the scrollbar's lane
              either way so nothing shifts — it otherwise paints over the
              last column's text on hover. */}
          <div
            className={cn(
              "min-w-0 flex-1 pr-2 [scrollbar-gutter:stable]",
              // Sideways scrolling stays while the panel is open, on a bar
              // that is always drawn rather than an overlay that fades:
              // with the panel covering the right of the table, that bar is
              // the only way back to the columns underneath it.
              selected
                ? "overflow-x-scroll overflow-y-hidden [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar]:h-2.5 [&::-webkit-scrollbar]:bg-transparent"
                : "overflow-y-auto",
            )}
          >
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>{TAGS_ROW_LABEL}</TableHead>
                  <TableHead>Tools</TableHead>
                  <TableHead>Where</TableHead>
                  <TableHead className="text-right">Updated</TableHead>
                  <TableHead>Status</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {groups.map((group) => (
                  <InstalledRow
                    key={group.key}
                    group={group}
                    selected={group.key === selectedKey}
                    onSelect={() =>
                      setSelectedKey(
                        group.key === selectedKey ? null : group.key,
                      )
                    }
                  />
                ))}
                {groups.length === 0 ? (
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
        <ItemDetail group={selected} onClose={() => setSelectedKey(null)} />
      </div>
    </div>
  );
}
