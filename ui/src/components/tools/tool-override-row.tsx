import { FolderOpen } from "lucide-react";
import { useState } from "react";
import type { HarnessId } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { toolName } from "@/lib/labels";
import { pickFolder } from "@/lib/pick-folder";

export function ToolOverrideRow({
  id,
  override,
  onSave,
}: {
  id: HarnessId;
  override: string;
  onSave: (root: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(override);
  const name = toolName(id);

  return (
    <div className="space-y-1.5 py-2.5">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <span className="text-sm font-medium text-foreground">{name}</span>{" "}
          {override ? (
            <span className="truncate font-mono text-xs text-muted-foreground">
              {override}
            </span>
          ) : null}
        </div>
        {editing ? null : (
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setDraft(override);
              setEditing(true);
            }}
          >
            {override ? "Change" : "Set override"}
          </Button>
        )}
      </div>
      {editing ? (
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            onSave(draft);
            setEditing(false);
          }}
        >
          <Input
            autoFocus
            className="max-w-md"
            placeholder="/path/to/folder"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
          />
          <Button
            type="button"
            variant="outline"
            size="icon"
            aria-label="Browse for folder"
            title="Browse…"
            onClick={() => {
              void pickFolder().then((picked) => {
                if (picked) setDraft(picked);
              });
            }}
          >
            <FolderOpen className="size-4" />
          </Button>
          <Button type="submit">Save</Button>
        </form>
      ) : null}
    </div>
  );
}
