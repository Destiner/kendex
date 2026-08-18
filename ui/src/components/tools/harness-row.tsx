import { Pencil } from "lucide-react";
import { useState } from "react";
import type { HarnessId, ItemKind } from "@/bindings";
import { KindCountBadges } from "@/components/kind-count-badges";
import { ToolIcon } from "@/components/tool-icon";
import { ToolFolderDialog } from "@/components/tools/tool-folder-dialog";
import { Button } from "@/components/ui/button";
import { NOT_INSTALLED_LABEL, TOOL_FOLDER_HELP } from "@/lib/copy";
import { toolName } from "@/lib/labels";
import { cn } from "@/lib/utils";
import { useNavStore } from "@/stores/nav";

/** One detected (or missing) AI coding tool, as a single compact row.
 *
 * Where the tool keeps its files is the row's second line, and changing it
 * is a pencil on that line — a page-long second list of the same seven
 * tools, one "Set folder" button each, said the same thing twice and buried
 * the fact that almost nobody needs it. */
export function HarnessRow({
  id,
  detectedRoot,
  version,
  counts,
  folder,
  onFolderChange,
}: {
  id: HarnessId;
  detectedRoot: string | null;
  version: string | null;
  counts: [ItemKind, number][];
  /** The folder this tool was pointed at by hand, when it was. */
  folder: string;
  onFolderChange: (root: string) => void;
}) {
  const goToLibrary = useNavStore((s) => s.goToLibrary);
  const [editing, setEditing] = useState(false);
  const name = toolName(id);

  return (
    <div className="group flex items-start justify-between gap-6 py-3.5">
      <div className="flex min-w-0 flex-col gap-1">
        <span className="flex items-center gap-2">
          <ToolIcon harness={id} muted={!detectedRoot} className="size-5" />
          <span
            className={cn(
              "text-sm font-medium",
              !detectedRoot && "text-muted-foreground",
            )}
          >
            {name}
          </span>
          {version ? (
            <span className="font-mono text-xs text-muted-foreground">
              {version}
            </span>
          ) : null}
        </span>
        <span className="flex min-w-0 items-center gap-1 pl-7">
          <span
            className={cn(
              "truncate text-[13px] text-muted-foreground",
              detectedRoot && "font-mono",
            )}
          >
            {detectedRoot ?? NOT_INSTALLED_LABEL}
          </span>
          {/* One pencil per row is one too many to look at seven times
              over; it appears on the row the pointer is on, and stays for
              the keyboard. */}
          <Button
            variant="quiet"
            size="icon-xs"
            className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
            aria-label={`Change where ${name} keeps its files`}
            title={TOOL_FOLDER_HELP}
            onClick={() => setEditing(true)}
          >
            <Pencil className="size-3" />
          </Button>
        </span>
      </div>
      <ToolFolderDialog
        open={editing}
        onOpenChange={setEditing}
        tool={name}
        folder={folder}
        detectedRoot={detectedRoot}
        onSave={onFolderChange}
      />
      {detectedRoot ? (
        <div className="flex shrink-0 flex-wrap justify-end gap-1.5 pt-0.5">
          <KindCountBadges
            counts={counts}
            onKindClick={(kind) => goToLibrary({ tool: id, kind })}
          />
        </div>
      ) : null}
    </div>
  );
}
