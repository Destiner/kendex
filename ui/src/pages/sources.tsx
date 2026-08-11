import { Plus, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Scope } from "@/bindings";
import { CatalogScopeGroup } from "@/components/catalog-scope";
import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { scopeLabel } from "@/lib/derive";
import { scopeName } from "@/lib/labels";
import { useSettingsStore } from "@/stores/settings";
import { useSourcesStore } from "@/stores/sources";

function sameScope(a: Scope, b: Scope): boolean {
  if (a.scope === "global" && b.scope === "global") return true;
  return a.scope === "project" && b.scope === "project" && a.root === b.root;
}

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
  const [targetScope, setTargetScope] = useState("global");
  const nameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void load();
  }, [load]);

  const scopes: Scope[] = [
    { scope: "global" },
    ...projects.map((root): Scope => ({ scope: "project", root })),
  ];

  const focusAddForm = () => {
    nameInputRef.current?.scrollIntoView({
      behavior: "smooth",
      block: "center",
    });
    nameInputRef.current?.focus();
  };

  return (
    <div>
      <PageHeader
        title="Catalogs"
        subtitle="Where skills, agents, and more come from"
        action={
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => void refreshRemotes()}
          >
            <RefreshCw className="size-4" /> Check for updates
          </Button>
        }
      />
      <div className="space-y-4 p-8">
        {error ? <p className="text-sm text-destructive">{error}</p> : null}
        {warnings.map((w) => (
          <p key={w} className="text-sm text-muted-foreground">
            Heads up: {w}
          </p>
        ))}

        {scopes.map((scope) => (
          <CatalogScopeGroup
            key={scopeLabel(scope)}
            scope={scope}
            rows={rows.filter((row) => sameScope(row.scope, scope))}
            busy={busy}
            onToggle={(name, enabled) => void toggle(scope, name, enabled)}
            onRemove={(name) => void remove(scope, name)}
            onAddFocus={focusAddForm}
          />
        ))}

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Add a catalog</CardTitle>
          </CardHeader>
          <CardContent>
            <form
              className="flex flex-wrap items-end gap-3"
              onSubmit={(e) => {
                e.preventDefault();
                const target =
                  scopes.find((s) => scopeLabel(s) === targetScope) ??
                  scopes[0];
                if (name.trim() && reference.trim()) {
                  void add(target, name.trim(), reference.trim()).then(() => {
                    setName("");
                    setReference("");
                  });
                }
              }}
            >
              <div className="space-y-1.5">
                <Label htmlFor="catalog-target">Add to</Label>
                <Select value={targetScope} onValueChange={setTargetScope}>
                  <SelectTrigger id="catalog-target" className="w-40">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {scopes.map((scope) => (
                      <SelectItem
                        key={scopeLabel(scope)}
                        value={scopeLabel(scope)}
                      >
                        {scopeName(scope)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="catalog-name">Name</Label>
                <Input
                  id="catalog-name"
                  ref={nameInputRef}
                  placeholder="team-tools"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="max-w-36"
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="catalog-location">Location</Label>
                <Input
                  id="catalog-location"
                  placeholder="owner/repo on GitHub, or a folder path"
                  value={reference}
                  onChange={(e) => setReference(e.target.value)}
                  className="max-w-80"
                />
              </div>
              <Button type="submit" disabled={busy}>
                <Plus className="size-4" /> Add catalog
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
