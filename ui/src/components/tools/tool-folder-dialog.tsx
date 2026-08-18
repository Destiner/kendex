import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { BROWSE_LABEL, TOOL_FOLDER_BODY, toolFolderTitle } from "@/lib/copy";
import { pickFolder } from "@/lib/pick-folder";

/**
 * Where one tool keeps its files. Type the path or pick it with the system's
 * own folder chooser — the same question either way, so it is one dialog
 * with two ways in rather than two controls that mean different things.
 */
export function ToolFolderDialog({
  open,
  onOpenChange,
  tool,
  folder,
  detectedRoot,
  onSave,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tool: string;
  /** The folder set by hand, when one was — otherwise empty. */
  folder: string;
  /** Where the tool was actually found, shown as the placeholder so the
   *  field says what leaving it empty means. */
  detectedRoot: string | null;
  onSave: (root: string) => void;
}) {
  const [draft, setDraft] = useState(folder);
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={(next) => {
        if (next) setDraft(folder);
        onOpenChange(next);
      }}
      title={toolFolderTitle(tool)}
      description={TOOL_FOLDER_BODY}
      confirmLabel="Save"
      onConfirm={() => {
        onSave(draft.trim());
        onOpenChange(false);
      }}
    >
      {/* One field, and a way out of typing it. Browse is a link, not a
          third button: the buttons in this dialog are the two answers to
          the question, and a matching one beside the field competes with
          them for the same weight. */}
      <div className="flex flex-col items-start gap-2">
        <Input
          autoFocus
          className="font-mono"
          placeholder={detectedRoot ?? "/path/to/folder"}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
        <Button
          type="button"
          variant="link"
          size="xs"
          className="px-0"
          onClick={() => {
            void pickFolder().then((picked) => {
              if (picked) setDraft(picked);
            });
          }}
        >
          <FolderOpen className="size-3" />
          {BROWSE_LABEL}
        </Button>
      </div>
    </ConfirmDialog>
  );
}
