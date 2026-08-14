import { FolderPlus, FolderSearch, Trash2 } from "lucide-react";
import { useState } from "react";
import type { ItemKind } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { ScopeCard } from "@/components/tools/scope-card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { countByKind } from "@/lib/derive";
import { kindLabel } from "@/lib/labels";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

function ProjectRow({
  root,
  counts,
  missing,
  onRemove,
}: {
  root: string;
  counts: [ItemKind, number][];
  missing: boolean;
  onRemove: () => void;
}) {
  const setScope = useNavStore((s) => s.setScope);
  const goToLibrary = useNavStore((s) => s.goToLibrary);
  const name = root.split("/").pop() ?? root;

  return (
    <div className="flex items-start justify-between gap-3 py-2.5">
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="truncate font-semibold">{name}</span>
          {missing ? (
            <Badge variant="destructive">Folder not found</Badge>
          ) : null}
        </div>
        <p className="truncate font-mono text-xs text-muted-foreground">
          {root}
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-1.5">
        {counts.length === 0 ? (
          <span className="text-xs text-muted-foreground">Nothing yet</span>
        ) : (
          counts.map(([kind, count]) => (
            <Badge
              key={kind}
              variant="outline"
              className="cursor-pointer hover:bg-accent"
              render={
                <button
                  type="button"
                  onClick={() => {
                    setScope({ project: root });
                    goToLibrary({ kind });
                  }}
                >
                  {count} {kindLabel(kind, count)}
                </button>
              }
            />
          ))
        )}
        <Button
          variant="ghost"
          size="icon"
          aria-label={`Stop tracking ${name}`}
          onClick={onRemove}
        >
          <Trash2 className="size-4" />
        </Button>
      </div>
    </div>
  );
}

/** "Projects": personal plus every registered project, with the add/scan tools. */
export function ProjectList() {
  const result = useScanStore((s) => s.result);
  const setScope = useNavStore((s) => s.setScope);
  const goToLibrary = useNavStore((s) => s.goToLibrary);
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
    <div className="p-8">
      <div className="mx-auto w-full max-w-5xl space-y-4">
        <ScopeCard
          title="Personal"
          subtitle="Just for you — works in every project on this computer"
          counts={[...countByKind(globalItems).entries()]}
          onKindClick={(kind) => {
            setScope("global");
            goToLibrary({ kind });
          }}
        />

        {projects.length === 0 ? (
          <div className="rounded-lg border px-4 py-6 text-center text-sm text-muted-foreground">
            No projects yet — add one below to manage its tools.
          </div>
        ) : (
          <div className="divide-y rounded-lg border px-4">
            {projects.map((root) => {
              const items =
                result?.items.filter(
                  (i) => i.scope.scope === "project" && i.scope.root === root,
                ) ?? [];
              const missing = result?.missingProjects.includes(root) ?? false;
              return (
                <ProjectRow
                  key={root}
                  root={root}
                  counts={[...countByKind(items).entries()]}
                  missing={missing}
                  onRemove={() => setRemoveTarget(root)}
                />
              );
            })}
          </div>
        )}

        <Card>
          <CardHeader>
            <CardTitle className="text-sm font-medium">Add a project</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <form
              className="flex items-end gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                if (addPath.trim()) {
                  void registerProject(addPath.trim()).then(() =>
                    setAddPath(""),
                  );
                }
              }}
            >
              <div className="max-w-md flex-1 space-y-1.5">
                <Label htmlFor="project-folder">Project folder</Label>
                <Input
                  id="project-folder"
                  placeholder="/path/to/project"
                  value={addPath}
                  onChange={(e) => setAddPath(e.target.value)}
                />
              </div>
              <Button type="submit">
                <FolderPlus className="size-4" /> Add
              </Button>
            </form>

            <div className="space-y-2 border-t pt-4">
              <p className="text-sm text-muted-foreground">
                Or scan a folder for projects
              </p>
              <form
                className="flex items-end gap-2"
                onSubmit={(e) => {
                  e.preventDefault();
                  if (discoverRoot.trim()) {
                    void discoverProjects(discoverRoot.trim()).then(setFound);
                  }
                }}
              >
                <div className="max-w-md flex-1 space-y-1.5">
                  <Label htmlFor="discover-folder">Folder to scan</Label>
                  <Input
                    id="discover-folder"
                    placeholder="/path/to/scan"
                    value={discoverRoot}
                    onChange={(e) => setDiscoverRoot(e.target.value)}
                  />
                </div>
                <Button type="submit" variant="outline">
                  <FolderSearch className="size-4" /> Scan
                </Button>
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
                        className="flex items-center justify-between gap-2 rounded-md border px-3 py-2 text-sm"
                      >
                        <span className="truncate font-mono text-xs">
                          {path}
                        </span>
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
    </div>
  );
}
