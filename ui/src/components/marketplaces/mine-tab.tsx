import { BookOpen, FolderOpen, Hammer, Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { commands } from "@/bindings";
import { EmptyState } from "@/components/empty-state";
import { TextBar } from "@/components/loading";
import { MarkdownView } from "@/components/markdown-view";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useMineStore } from "@/stores/mine";
import { MineCreateDialog } from "./mine-create-dialog";
import { MineImportDialog } from "./mine-import-dialog";
import { MineRowCard } from "./mine-row";

/** The marketplaces the user authors. Rows are computed fresh from each
 * folder; a folder that stopped reading keeps its place with the reason. */
export function MineTab() {
  const rows = useMineStore((s) => s.rows);
  const load = useMineStore((s) => s.load);
  const registerExisting = useMineStore((s) => s.useExisting);
  const actionError = useMineStore((s) => s.actionError);
  const [creating, setCreating] = useState(false);
  const [importTarget, setImportTarget] = useState<string | null>(null);
  const [doc, setDoc] = useState<string | null>(null);
  const [docOpen, setDocOpen] = useState(false);

  useEffect(() => {
    void load();
  }, [load]);

  const pickExisting = () => {
    void commands.pickFolder().then((picked) => {
      if (picked.status === "ok" && picked.data) {
        void registerExisting(picked.data);
      }
    });
  };

  const openDoc = () => {
    setDocOpen(true);
    if (doc === null) {
      void commands.mineAuthoringDoc().then((text) => setDoc(text));
    }
  };

  const actions = (
    <div className="flex gap-2">
      <Button onClick={() => setCreating(true)}>
        <Plus className="size-4" /> Create a marketplace…
      </Button>
      <Button variant="outline" onClick={pickExisting}>
        <FolderOpen className="size-4" /> Use an existing folder…
      </Button>
      <Button variant="ghost" onClick={openDoc}>
        <BookOpen className="size-4" /> How a marketplace repo works
      </Button>
    </div>
  );

  return (
    <div className="space-y-3">
      {rows === null ? (
        <div className="space-y-3">
          <TextBar width="w-48" title />
          <TextBar width="w-72" />
        </div>
      ) : rows.length === 0 ? (
        <EmptyState
          icon={Hammer}
          title="Nothing you publish yet"
          action={actions}
        >
          Build a marketplace from skills and agents you already have, or start
          an empty one and add to it later.
        </EmptyState>
      ) : (
        <>
          <div className="flex justify-end">{actions}</div>
          {rows.map((entry) =>
            entry.state === "ready" ? (
              <MineRowCard
                key={entry.row.path}
                row={entry.row}
                onImport={(path) => setImportTarget(path)}
              />
            ) : (
              <div
                key={entry.path}
                className="rounded-lg border border-border p-4 text-sm"
              >
                <p className="truncate font-mono text-xs text-muted-foreground">
                  {entry.path}
                </p>
                <p className="text-critical">{entry.why}</p>
              </div>
            ),
          )}
        </>
      )}
      {actionError && !creating && importTarget === null ? (
        <p className="text-sm text-critical" role="alert">
          {actionError}
        </p>
      ) : null}
      <Dialog open={docOpen} onOpenChange={setDocOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>How a marketplace repo works</DialogTitle>
          </DialogHeader>
          <ScrollArea className="max-h-[70vh] pr-3">
            {doc === null ? (
              <TextBar width="w-64" />
            ) : (
              <MarkdownView source={doc} />
            )}
          </ScrollArea>
        </DialogContent>
      </Dialog>
      <MineCreateDialog open={creating} onOpenChange={setCreating} />
      <MineImportDialog
        target={importTarget ?? ""}
        open={importTarget !== null}
        onOpenChange={(open) => {
          if (!open) setImportTarget(null);
        }}
      />
    </div>
  );
}
