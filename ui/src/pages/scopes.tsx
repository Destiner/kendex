import { FolderPlus, FolderSearch, Trash2 } from "lucide-react";
import { useState } from "react";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { countByKind } from "@/lib/derive";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

export function ScopesPage() {
  const result = useScanStore((s) => s.result);
  const { settings, registerProject, unregisterProject, discoverProjects } =
    useSettingsStore();
  const [addPath, setAddPath] = useState("");
  const [discoverRoot, setDiscoverRoot] = useState("");
  const [found, setFound] = useState<string[] | null>(null);

  const globalItems =
    result?.items.filter((i) => i.scope.scope === "global") ?? [];
  const projects = settings?.projects ?? [];

  return (
    <div>
      <PageHeader
        title="Scopes"
        subtitle="Global and every registered project"
      />
      <div className="space-y-4 p-8">
        <ScopeCard
          title="Global"
          counts={[...countByKind(globalItems).entries()]}
        />

        {projects.map((root) => {
          const items =
            result?.items.filter(
              (i) => i.scope.scope === "project" && i.scope.root === root,
            ) ?? [];
          const missing = result?.missingProjects.includes(root) ?? false;
          return (
            <ScopeCard
              key={root}
              title={root}
              missing={missing}
              counts={[...countByKind(items).entries()]}
              onRemove={() => void unregisterProject(root)}
            />
          );
        })}

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Add projects</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <form
              className="flex gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                if (addPath.trim()) {
                  void registerProject(addPath.trim()).then(() =>
                    setAddPath(""),
                  );
                }
              }}
            >
              <Input
                placeholder="/path/to/project"
                value={addPath}
                onChange={(e) => setAddPath(e.target.value)}
              />
              <Button type="submit" variant="outline">
                <FolderPlus className="size-4" /> Register
              </Button>
            </form>
            <form
              className="flex gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                if (discoverRoot.trim()) {
                  void discoverProjects(discoverRoot.trim()).then(setFound);
                }
              }}
            >
              <Input
                placeholder="Walk this directory for projects…"
                value={discoverRoot}
                onChange={(e) => setDiscoverRoot(e.target.value)}
              />
              <Button type="submit" variant="outline">
                <FolderSearch className="size-4" /> Discover
              </Button>
            </form>
            {found ? (
              <div className="space-y-1">
                {found.length === 0 ? (
                  <p className="text-sm text-muted-foreground">
                    no projects found
                  </p>
                ) : (
                  found.map((path) => (
                    <div
                      key={path}
                      className="flex items-center justify-between text-sm"
                    >
                      <span className="break-all">{path}</span>
                      <Button
                        variant="ghost"
                        size="sm"
                        disabled={projects.includes(path)}
                        onClick={() => void registerProject(path)}
                      >
                        {projects.includes(path) ? "registered" : "register"}
                      </Button>
                    </div>
                  ))
                )}
              </div>
            ) : null}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function ScopeCard({
  title,
  counts,
  missing,
  onRemove,
}: {
  title: string;
  counts: [string, number][];
  missing?: boolean;
  onRemove?: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <span className="break-all">{title}</span>
          {missing ? <Badge variant="destructive">missing</Badge> : null}
          {onRemove ? (
            <Button
              variant="ghost"
              size="icon"
              className="ml-auto"
              aria-label="Unregister"
              onClick={onRemove}
            >
              <Trash2 className="size-4" />
            </Button>
          ) : null}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex flex-wrap gap-1.5">
        {counts.length === 0 ? (
          <span className="text-sm text-muted-foreground">
            nothing observed
          </span>
        ) : (
          counts.map(([kind, count]) => (
            <Badge key={kind} variant="outline">
              {kind} {count}
            </Badge>
          ))
        )}
      </CardContent>
    </Card>
  );
}
