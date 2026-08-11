import { GitBranch, Plus, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { Scope, SourceRow } from "@/bindings";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { scopeLabel } from "@/lib/derive";
import { useSettingsStore } from "@/stores/settings";
import { useSourcesStore } from "@/stores/sources";

export function SourcesPage() {
  const {
    rows,
    busy,
    error,
    warnings,
    load,
    add,
    remove,
    toggle,
    refreshRemotes,
  } = useSourcesStore();
  const projects = useSettingsStore((s) => s.settings?.projects ?? []);
  const [name, setName] = useState("");
  const [reference, setReference] = useState("");
  const [targetScope, setTargetScope] = useState<string>("global");

  useEffect(() => {
    void load();
  }, [load]);

  const scopes: { label: string; scope: Scope }[] = [
    { label: "Global", scope: { scope: "global" } },
    ...projects.map((root) => ({
      label: root.split("/").pop() ?? root,
      scope: { scope: "project", root } as Scope,
    })),
  ];
  const byScope = new Map<string, SourceRow[]>();
  for (const row of rows) {
    const key = scopeLabel(row.scope);
    byScope.set(key, [...(byScope.get(key) ?? []), row]);
  }

  return (
    <div>
      <PageHeader
        title="Sources"
        subtitle="Catalogs each scope installs from"
      />
      <div className="space-y-4 p-8">
        {error ? <p className="text-sm text-destructive">{error}</p> : null}
        {warnings.map((w) => (
          <p key={w} className="text-sm text-muted-foreground">
            warning: {w}
          </p>
        ))}
        <div className="flex gap-2">
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => void refreshRemotes()}
          >
            <RefreshCw className="size-4" /> Refresh remotes
          </Button>
        </div>

        {[...byScope.entries()].map(([label, sourceRows]) => (
          <Card key={label}>
            <CardHeader>
              <CardTitle className="break-all text-base">{label}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {sourceRows.map((row) => (
                <div key={row.name} className="flex items-center gap-3 text-sm">
                  <GitBranch className="size-4 shrink-0 text-muted-foreground" />
                  <span className="font-medium">{row.name}</span>
                  <span className="break-all text-muted-foreground">
                    {row.reference}
                  </span>
                  {row.head ? (
                    <Badge variant="outline">@{row.head}</Badge>
                  ) : null}
                  <Badge variant="secondary">
                    {row.declaredItems.length} item(s)
                  </Badge>
                  <span className="ml-auto flex items-center gap-2">
                    <Switch
                      checked={row.enabled}
                      disabled={busy}
                      onCheckedChange={(checked) =>
                        void toggle(row.scope, row.name, checked)
                      }
                    />
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={busy || row.declaredItems.length > 0}
                      title={
                        row.declaredItems.length > 0
                          ? `still referenced by ${row.declaredItems.join(", ")} — disable instead`
                          : "remove"
                      }
                      onClick={() => void remove(row.scope, row.name)}
                    >
                      Remove
                    </Button>
                  </span>
                </div>
              ))}
              {sourceRows.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  no sources declared
                </p>
              ) : null}
            </CardContent>
          </Card>
        ))}

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Add a source</CardTitle>
          </CardHeader>
          <CardContent>
            <form
              className="flex flex-wrap gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                const target = scopes.find(
                  (s) => scopeLabel(s.scope) === targetScope,
                )?.scope ?? {
                  scope: "global",
                };
                if (name.trim() && reference.trim()) {
                  void add(target, name.trim(), reference.trim()).then(() => {
                    setName("");
                    setReference("");
                  });
                }
              }}
            >
              <select
                className="h-9 rounded-md border bg-background px-2 text-sm"
                value={targetScope}
                onChange={(e) => setTargetScope(e.target.value)}
              >
                {scopes.map(({ label, scope }) => (
                  <option key={scopeLabel(scope)} value={scopeLabel(scope)}>
                    {label}
                  </option>
                ))}
              </select>
              <Input
                placeholder="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="max-w-36"
              />
              <Input
                placeholder="owner/repo or /path/to/catalog"
                value={reference}
                onChange={(e) => setReference(e.target.value)}
                className="max-w-80"
              />
              <Button type="submit" variant="outline" disabled={busy}>
                <Plus className="size-4" /> Add
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
