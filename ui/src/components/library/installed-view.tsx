import { useEffect, useMemo, useRef, useState } from "react";
import type { ItemKind } from "@/bindings";
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
import {
  filterItems,
  groupItems,
  projectScopes,
  type ScopeSelection,
} from "@/lib/derive";
import { isSearchShortcutKey } from "@/lib/search-shortcut";
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
  const [where, setWhere] = useState<string>("any");
  const [search, setSearch] = useState("");
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const projects = result ? projectScopes(result) : [];

  // The filter is a one-time handoff from wherever the link was clicked
  // (Tools, Projects); once applied, further tab visits start from "any"
  // again rather than reapplying a stale filter.
  useEffect(() => {
    clearLibraryFilter();
  }, [clearLibraryFilter]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!isSearchShortcutKey(event.key, event.target as HTMLElement | null))
        return;
      event.preventDefault();
      searchRef.current?.focus();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const effectiveScope: ScopeSelection =
    where === "any"
      ? scope
      : where === "global"
        ? "global"
        : { project: where };

  const groups = useMemo(() => {
    if (!result) return [];
    const filtered = filterItems(result.items, {
      scope: effectiveScope,
      kind: kind === "any" ? undefined : (kind as ItemKind),
      harness: harness === "any" ? undefined : harness,
      search,
    });
    return groupItems(filtered);
  }, [result, effectiveScope, kind, harness, search]);

  const selected = groups.find((g) => g.key === selectedKey) ?? null;
  const hasAnyItems = (result?.items.length ?? 0) > 0;

  const clearFilters = () => {
    setKind("any");
    setHarness("any");
    setWhere("any");
    setSearch("");
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <LibraryFilters
        searchRef={searchRef}
        search={search}
        onSearchChange={setSearch}
        kind={kind}
        onKindChange={setKind}
        harness={harness}
        onHarnessChange={setHarness}
        where={where}
        onWhereChange={setWhere}
        projects={projects}
      />
      <div className="mx-auto flex min-h-0 w-full max-w-5xl flex-1">
        <div className="min-w-0 flex-1 overflow-y-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Type</TableHead>
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
                    setSelectedKey(group.key === selectedKey ? null : group.key)
                  }
                />
              ))}
              {groups.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} className="py-10">
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
                            Add a catalog to start installing skills and agents.
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
        {selected ? (
          <ItemDetail group={selected} onClose={() => setSelectedKey(null)} />
        ) : null}
      </div>
    </div>
  );
}
