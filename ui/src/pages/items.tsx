import { useMemo, useState } from "react";
import type { ItemKind } from "@/bindings";
import { PageHeader } from "@/components/page-header";
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
import {
  filterItems,
  groupItems,
  type ItemGroup,
  scopeLabel,
} from "@/lib/derive";
import { cn } from "@/lib/utils";
import { useAuditStore } from "@/stores/audit";
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
const HARNESSES = ["claude", "codex", "opencode", "cursor", "pi"];

export function ItemsPage() {
  const result = useScanStore((s) => s.result);
  const scope = useNavStore((s) => s.scope);
  const [kind, setKind] = useState<string>("any");
  const [harness, setHarness] = useState<string>("any");
  const [search, setSearch] = useState("");
  const [selectedKey, setSelectedKey] = useState<string | null>(null);

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

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Items"
        subtitle="Everything observed, grouped by logical item"
      />
      <div className="flex gap-2 border-b px-8 py-3">
        <Input
          placeholder="Search…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="max-w-56"
        />
        <Select value={kind} onValueChange={setKind}>
          <SelectTrigger className="w-36">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="any">Any kind</SelectItem>
            {KINDS.map((k) => (
              <SelectItem key={k} value={k}>
                {k}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={harness} onValueChange={setHarness}>
          <SelectTrigger className="w-36">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="any">Any harness</SelectItem>
            {HARNESSES.map((h) => (
              <SelectItem key={h} value={h}>
                {h}
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
                <TableHead>Kind</TableHead>
                <TableHead>Harnesses</TableHead>
                <TableHead>State</TableHead>
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
                    {group.kind}
                  </TableCell>
                  <TableCell>
                    <span className="flex flex-wrap gap-1">
                      {group.harnesses.map((h) => (
                        <Badge key={h} variant="outline">
                          {h}
                        </Badge>
                      ))}
                      {group.shared ? (
                        <Badge variant="secondary">shared</Badge>
                      ) : null}
                    </span>
                  </TableCell>
                  <TableCell>
                    {group.installations.some((i) => i.enabled === false) ? (
                      <Badge variant="secondary">disabled</Badge>
                    ) : null}
                  </TableCell>
                </TableRow>
              ))}
              {groups.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={4}
                    className="py-10 text-center text-muted-foreground"
                  >
                    Nothing matches
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

function ItemDetail({ group }: { group: ItemGroup }) {
  const { busy, toggle, removeItem } = useAuditStore();
  const managed = group.kind === "agent" || group.kind === "skill";
  const anyDisabled = group.installations.some((i) => i.enabled === false);
  const scope = group.installations[0]?.scope;
  return (
    <aside className="w-96 shrink-0 overflow-y-auto border-l p-5">
      <h2 className="font-semibold">{group.name}</h2>
      <p className="mb-4 text-sm text-muted-foreground">{group.kind}</p>
      {managed && scope ? (
        <div className="mb-4 flex gap-2">
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => void toggle(scope, group.name, anyDisabled)}
          >
            {anyDisabled ? "Enable" : "Disable"}
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => void removeItem(scope, group.name)}
          >
            Remove
          </Button>
        </div>
      ) : null}
      <div className="space-y-4">
        {group.installations.map((install) => (
          <div
            key={`${install.harness}:${scopeLabel(install.scope)}:${install.path}`}
            className="rounded-md border p-3 text-sm"
          >
            <div className="mb-1 flex items-center gap-2">
              <Badge variant="outline">{install.harness}</Badge>
              <span className="text-xs text-muted-foreground">
                {scopeLabel(install.scope)}
              </span>
              {install.enabled === false ? (
                <Badge variant="secondary">disabled</Badge>
              ) : null}
            </div>
            <p className="break-all text-xs text-muted-foreground">
              {install.path}
            </p>
            {install.fileState.state === "symlink" ? (
              <p className="mt-1 break-all text-xs text-muted-foreground">
                → {install.fileState.target}
                {install.fileState.broken ? " (broken)" : ""}
              </p>
            ) : null}
            {install.origin ? (
              <p className="mt-1 break-all text-xs text-muted-foreground">
                from {install.origin}
              </p>
            ) : null}
          </div>
        ))}
      </div>
    </aside>
  );
}
