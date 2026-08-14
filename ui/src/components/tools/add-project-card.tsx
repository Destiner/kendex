import { FolderOpen, FolderPlus, FolderSearch } from "lucide-react";
import { useState } from "react";
import { SectionLabel } from "@/components/card-section";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { pickFolder } from "@/lib/pick-folder";

/** The "Add a project" card: type a path or scan a folder for candidates. */
export function AddProjectCard({
  projects,
  registerProject,
  discoverProjects,
}: {
  projects: string[];
  registerProject: (path: string) => Promise<boolean>;
  discoverProjects: (root: string) => Promise<string[]>;
}) {
  const [addPath, setAddPath] = useState("");
  const [adding, setAdding] = useState(false);
  const [discoverRoot, setDiscoverRoot] = useState("");
  const [found, setFound] = useState<string[] | null>(null);

  return (
    <Card className="gap-3 py-4">
      <CardHeader className="gap-1">
        <SectionLabel>Add a project</SectionLabel>
      </CardHeader>
      <CardContent className="space-y-4">
        <form
          className="flex items-end gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            const path = addPath.trim();
            if (!path || adding) return;
            setAdding(true);
            void registerProject(path).then((ok) => {
              setAdding(false);
              if (ok) setAddPath("");
            });
          }}
        >
          <div className="max-w-md flex-1 space-y-1.5">
            <Label htmlFor="project-folder">Project folder</Label>
            <Input
              id="project-folder"
              placeholder="/path/to/project"
              value={addPath}
              disabled={adding}
              onChange={(e) => setAddPath(e.target.value)}
            />
          </div>
          <Button
            type="button"
            variant="outline"
            size="icon"
            aria-label="Browse for project folder"
            title="Browse…"
            disabled={adding}
            onClick={() => {
              void pickFolder().then((picked) => {
                if (picked) setAddPath(picked);
              });
            }}
          >
            <FolderOpen className="size-4" />
          </Button>
          <Button type="submit" disabled={adding}>
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
            <Button
              type="button"
              variant="outline"
              size="icon"
              aria-label="Browse for folder to scan"
              title="Browse…"
              onClick={() => {
                void pickFolder().then((picked) => {
                  if (picked) setDiscoverRoot(picked);
                });
              }}
            >
              <FolderOpen className="size-4" />
            </Button>
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
                    <span className="truncate font-mono text-xs">{path}</span>
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
  );
}
