import { useState } from "react";
import { PathField } from "@/components/tools/path-field";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ADD_PROJECT_HELP } from "@/lib/copy";

/** Point vstack at one folder. */
export function AddProjectDialog({
  open,
  onOpenChange,
  registerProject,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  registerProject: (path: string) => Promise<boolean>;
}) {
  const [path, setPath] = useState("");
  const [adding, setAdding] = useState(false);

  const submit = () => {
    const trimmed = path.trim();
    if (!trimmed || adding) return;
    setAdding(true);
    void registerProject(trimmed).then((ok) => {
      setAdding(false);
      // A rejected path keeps the dialog open with what was typed still in
      // it — the error surfaces behind, and retyping a long path is worse
      // than reading it again.
      if (ok) {
        setPath("");
        onOpenChange(false);
      }
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add a project</DialogTitle>
          <DialogDescription>{ADD_PROJECT_HELP}</DialogDescription>
        </DialogHeader>
        <form
          className="flex flex-col gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <PathField
            id="project-folder"
            placeholder="/path/to/project"
            value={path}
            onChange={setPath}
            disabled={adding}
            browseLabel="Browse for a project folder"
          />
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={adding || !path.trim()}>
              Add project
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
