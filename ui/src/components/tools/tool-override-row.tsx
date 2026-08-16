import { FolderOpen } from "lucide-react";
import { useState } from "react";
import type { HarnessId } from "@/bindings";
import { SettingRow } from "@/components/section";
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

  return (
    <div>
      <SettingRow
        label={toolName(id)}
        description={
          override ? (
            <span className="font-mono break-all">{override}</span>
          ) : undefined
        }
      >
        {editing ? null : (
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setDraft(override);
              setEditing(true);
            }}
          >
            {override ? "Change" : "Set folder"}
          </Button>
        )}
      </SettingRow>
      {editing ? (
        <form
          className="flex gap-2 pb-3.5"
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
          <Button
            type="button"
            variant="ghost"
            onClick={() => setEditing(false)}
          >
            Cancel
          </Button>
        </form>
      ) : null}
    </div>
  );
}
