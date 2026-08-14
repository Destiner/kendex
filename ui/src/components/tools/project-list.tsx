import { FolderPlus, FolderSearch } from "lucide-react";
import { useState } from "react";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { ScopeCard } from "@/components/tools/scope-card";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { countByKind } from "@/lib/derive";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

/** "Projects": personal plus every registered project, with the add/scan tools. */
export function ProjectList() {
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
    <div className="space-y-4 p-8">
      <ScopeCard
        title="Personal"
        subtitle="Just for you — works in every project on this computer"
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
                void registerProject(addPath.trim()).then(() => setAddPath(""));
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
