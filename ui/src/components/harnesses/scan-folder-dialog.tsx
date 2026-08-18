import { FolderSearch } from "lucide-react";
import { useState } from "react";
import { PathField } from "@/components/harnesses/path-field";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { NO_PROJECTS_FOUND, SCAN_FOLDER_HELP } from "@/lib/copy";

/** Look through a folder and add whichever repositories turn up. */
export function ScanFolderDialog({
  open,
  onOpenChange,
  projects,
  registerProject,
  discoverProjects,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projects: string[];
  registerProject: (path: string) => Promise<boolean>;
  discoverProjects: (root: string) => Promise<string[]>;
}) {
  const [root, setRoot] = useState("");
  const [found, setFound] = useState<string[] | null>(null);
  const [scanning, setScanning] = useState(false);

  const close = (next: boolean) => {
    if (!next) setFound(null);
    onOpenChange(next);
  };

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Scan a folder</DialogTitle>
          <DialogDescription>{SCAN_FOLDER_HELP}</DialogDescription>
        </DialogHeader>
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            const trimmed = root.trim();
            if (!trimmed || scanning) return;
            setScanning(true);
            void discoverProjects(trimmed).then((paths) => {
              setFound(paths);
              setScanning(false);
            });
          }}
        >
          <PathField
            id="discover-folder"
            placeholder="/path/to/scan"
            value={root}
            onChange={setRoot}
            disabled={scanning}
            browseLabel="Browse for a folder to scan"
          />
          <Button
            type="submit"
            variant="outline"
            disabled={scanning || !root.trim()}
          >
            <FolderSearch className="size-4" />
            {scanning ? "Scanning…" : "Scan"}
          </Button>
        </form>
        {found ? (
          <div className="max-h-64 overflow-y-auto">
            {found.length === 0 ? (
              <p className="text-[13px] text-muted-foreground">
                {NO_PROJECTS_FOUND}
              </p>
            ) : (
              found.map((path) => (
                <div
                  key={path}
                  className="flex items-center justify-between gap-3 border-b border-border/40 py-2 last:border-0"
                >
                  <span className="truncate font-mono text-[13px] text-muted-foreground">
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
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => close(false)}>
            Done
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
