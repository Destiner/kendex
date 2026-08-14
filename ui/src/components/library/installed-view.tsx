import { useEffect, useMemo, useRef, useState } from "react";
import type { HarnessId, ItemKind } from "@/bindings";
import { ItemDetail } from "@/components/item-detail";
import { LibraryFilters } from "@/components/library/library-filters";
import { StatusDot } from "@/components/status-dot";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { filterItems, groupItems } from "@/lib/derive";
import { kindLabel, toolName } from "@/lib/labels";
import { isSearchShortcutKey } from "@/lib/search-shortcut";
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
  const [search, setSearch] = useState("");
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

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

  const groups = useMemo(() => {
    if (!result) return [];
    const filtered = filterItems(result.items, {
      scope,
      kind: kind === "any" ? undefined : (kind as ItemKind),
      harness: harness === "any" ? undefined : harness,
      search,
    });
    return groupItems(filtered);
  }, [result, scope, kind, harness, search]);

  const selected = groups.find((g) => g.key === selectedKey) ?? null;
  const hasAnyItems = (result?.items.length ?? 0) > 0;

  const clearFilters = () => {
    setKind("any");
    setHarness("any");
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
      />
      <div className="mx-auto flex min-h-0 w-full max-w-5xl flex-1">
        <div className="min-w-0 flex-1 overflow-y-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Tools</TableHead>
                <TableHead>Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {groups.map((group) => (
                <TableRow
                  key={group.key}
                  onClick={() =>
                    setSelectedKey(group.key === selectedKey ? null : group.key)
                  }
                  className={cn(
                    "cursor-pointer",
                    group.key === selectedKey && "bg-muted/60",
                  )}
                >
                  <TableCell className="font-medium">
                    {group.name}
                    {group.description ? (
                      <p className="max-w-96 truncate text-xs font-normal text-muted-foreground">
                        {group.description}
                      </p>
                    ) : null}
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
              ))}
              {groups.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4} className="py-10">
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
        {selected ? <ItemDetail group={selected} /> : null}
      </div>
    </div>
  );
}
