import { FolderPlus, FolderSearch, Trash2 } from "lucide-react";
import { useState } from "react";
import type { ItemKind } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { countByKind } from "@/lib/derive";
import { kindLabel } from "@/lib/labels";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

export function ScopesPage() {
  const result = useScanStore((s) => s.result);
  const { settings, registerProject, unregisterProject, discoverProjects } =
    useSettingsStore();
  const [addPath, setAddPath] = useState("");
  const [discoverRoot, setDiscoverRoot] = useState("");
  const [found, setFound] = useState<string[] | null>(null);
  const [removeTarget, setRemoveTarget] = useState<string | null>(null);

  const globalItems =
    result?.items.filter((i) => i.scope.scope === "global") ?? [];
  const projects = settings?.projects ?? [];

  return (
    <div>
      <PageHeader title="Projects" subtitle="Where your setup applies" />
      <div className="space-y-4 p-8">
        <ScopeCard
          title="Global"
          subtitle="Applies everywhere on this machine"
          counts={[...countByKind(globalItems).entries()]}
        />

        {projects.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No projects yet. Add one to manage its tools.
          </p>
        ) : null}

        {projects.map((root) => {
          const items =
            result?.items.filter(
              (i) => i.scope.scope === "project" && i.scope.root === root,
            ) ?? [];
          const missing = result?.missingProjects.includes(root) ?? false;
          return (
            <ScopeCard
              key={root}
              title={root.split("/").pop() ?? root}
              path={root}
              missing={missing}
              counts={[...countByKind(items).entries()]}
              onRemove={() => setRemoveTarget(root)}
            />
          );
        })}

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Add a project</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <form
              className="space-y-1.5"
              onSubmit={(e) => {
                e.preventDefault();
                if (addPath.trim()) {
                  void registerProject(addPath.trim()).then(() =>
                    setAddPath(""),
                  );
                }
              }}
            >
              <Label htmlFor="project-folder">Project folder</Label>
              <div className="flex gap-2">
                <Input
                  id="project-folder"
                  placeholder="/path/to/project"
                  value={addPath}
                  onChange={(e) => setAddPath(e.target.value)}
                />
                <Button type="submit">
                  <FolderPlus className="size-4" /> Add
                </Button>
              </div>
            </form>

            <div className="space-y-1.5 border-t pt-4">
              <p className="text-sm text-muted-foreground">
                Or scan a folder for projects
              </p>
              <form
                className="space-y-1.5"
                onSubmit={(e) => {
                  e.preventDefault();
                  if (discoverRoot.trim()) {
                    void discoverProjects(discoverRoot.trim()).then(setFound);
                  }
                }}
              >
                <Label htmlFor="discover-folder">Folder to scan</Label>
                <div className="flex gap-2">
                  <Input
                    id="discover-folder"
                    placeholder="/path/to/scan"
                    value={discoverRoot}
                    onChange={(e) => setDiscoverRoot(e.target.value)}
                  />
                  <Button type="submit" variant="outline">
                    <FolderSearch className="size-4" /> Scan
                  </Button>
                </div>
              </form>
              {found ? (
                <div className="space-y-1 pt-1">
                  {found.length === 0 ? (
                    <p className="text-sm text-muted-foreground">
                      No projects found there.
                    </p>
                  ) : (
                    found.map((path) => (
                      <div
                        key={path}
                        className="flex items-center justify-between gap-2 text-sm"
                      >
                        <span className="break-all">{path}</span>
                        <Button
                          variant="outline"
                          size="sm"
                          disabled={projects.includes(path)}
                          onClick={() => void registerProject(path)}
                        >
                          {projects.includes(path) ? "Added" : "Add"}
                        </Button>
                      </div>
                    ))
                  )}
                </div>
              ) : null}
            </div>
          </CardContent>
        </Card>
      </div>

      <ConfirmDialog
        open={removeTarget !== null}
        onOpenChange={(open) => {
          if (!open) setRemoveTarget(null);
        }}
        title={`Stop tracking ${removeTarget?.split("/").pop() ?? ""}?`}
        description="vstack will stop managing this project. Nothing in the folder is deleted."
        confirmLabel="Stop tracking"
        destructive
        onConfirm={() => {
          if (removeTarget) void unregisterProject(removeTarget);
          setRemoveTarget(null);
        }}
      />
    </div>
  );
}

function ScopeCard({
  title,
  path,
  counts,
  missing,
  subtitle,
  onRemove,
}: {
  title: string;
  path?: string;
  counts: [ItemKind, number][];
  missing?: boolean;
  subtitle?: string;
  onRemove?: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-start gap-2 text-base">
          <div className="min-w-0">
            <span className="break-all font-semibold">{title}</span>
            {path ? (
              <p className="truncate font-mono text-xs font-normal text-muted-foreground">
                {path}
              </p>
            ) : null}
          </div>
          {missing ? (
            <Badge variant="destructive">Folder not found</Badge>
          ) : null}
          {onRemove ? (
            <Button
              variant="ghost"
              size="icon"
              className="ml-auto shrink-0"
              aria-label={`Stop tracking ${title}`}
              onClick={onRemove}
            >
              <Trash2 className="size-4" />
            </Button>
          ) : null}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {subtitle ? (
          <p className="text-sm text-muted-foreground">{subtitle}</p>
        ) : null}
        {missing ? (
          <p className="text-sm text-muted-foreground">
            This folder could not be found on this machine.
          </p>
        ) : null}
        <div className="flex flex-wrap gap-1.5">
          {counts.length === 0 ? (
            <span className="text-sm text-muted-foreground">
              Nothing from vstack yet.
            </span>
          ) : (
            counts.map(([kind, count]) => (
              <Badge key={kind} variant="outline">
                {count} {kindLabel(kind, count)}
              </Badge>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
