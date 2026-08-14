import { useEffect, useMemo, useRef, useState } from "react";
import type { HarnessId, ItemKind } from "@/bindings";
import { ItemDetail } from "@/components/item-detail";
import { StatusDot } from "@/components/status-dot";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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

const KINDS: ItemKind[] = [
  "agent",
  "skill",
  "hook",
  "command",
  "mcp-server",
  "plugin",
  "pi-extension",
];
const HARNESSES: HarnessId[] = [
  "claude",
  "codex",
  "opencode",
  "cursor",
  "pi",
  "gemini",
  "copilot",
];

/** "Installed": everything on this machine, filterable, with a detail pane. */
export function InstalledView() {
  const result = useScanStore((s) => s.result);
  const scope = useNavStore((s) => s.scope);
  const goToLibrary = useNavStore((s) => s.goToLibrary);
  const [kind, setKind] = useState<string>("any");
  const [harness, setHarness] = useState<string>("any");
  const [search, setSearch] = useState("");
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

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
      <div className="flex gap-2 border-b px-8 py-3">
        <div className="relative max-w-56 flex-1">
          <Input
            ref={searchRef}
            placeholder="Search by name…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pr-8"
          />
          <kbd className="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
            /
          </kbd>
        </div>
        <Select value={kind} onValueChange={setKind}>
          <SelectTrigger className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="any">All types</SelectItem>
            {KINDS.map((k) => (
              <SelectItem key={k} value={k}>
                {kindLabel(k, 2)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={harness} onValueChange={setHarness}>
          <SelectTrigger className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="any">All tools</SelectItem>
            {HARNESSES.map((h) => (
              <SelectItem key={h} value={h}>
                {toolName(h)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="flex min-h-0 flex-1">
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
                        <p className="text-muted-foreground">
                          Nothing matches.
                        </p>
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
                          <p className="text-muted-foreground">
                            Nothing installed yet.
                          </p>
                          <p className="text-sm text-muted-foreground">
                            Add a catalog to start installing skills and agents.
                          </p>
                        </div>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => goToLibrary("add")}
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
