import { ExternalLink, X } from "lucide-react";
import { useEffect, useState } from "react";
import { commands } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { ItemDetailMeta } from "@/components/item-detail-meta";
import { ItemPreview } from "@/components/library/item-preview";
import { ReportDialog } from "@/components/report-dialog";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ItemGroup } from "@/lib/derive";
import { editorOpenPath } from "@/lib/editor-path";
import { kindIcon } from "@/lib/kind-icon";
import {
  EDITOR_ERROR_STEPS,
  EDITOR_ERROR_TITLE,
  FILE_BROWSER_ERROR_TITLE,
  hookDisplayName,
  OPEN_IN_EDITOR_LABEL,
  OPEN_IN_FILE_BROWSER_LABEL,
  OPEN_IN_LABEL,
} from "@/lib/labels";
import { cn } from "@/lib/utils";
import { useAuditStore } from "@/stores/audit";
import { useProblemsStore } from "@/stores/problems";

export function ItemDetail({
  group,
  onClose,
}: {
  // null closes the flyout — kept mounted (not conditionally rendered) so
  // closing plays the slide-out transition instead of vanishing, and so
  // switching the selected row just swaps content instead of remounting.
  group: ItemGroup | null;
  onClose: () => void;
}) {
  const { busy, toggle, removeItem } = useAuditStore();
  const showError = useProblemsStore((s) => s.showError);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [lastGroup, setLastGroup] = useState<ItemGroup | null>(null);
  const open = group != null;

  useEffect(() => {
    if (group) setLastGroup(group);
  }, [group]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  // Nothing has ever been selected this session — no panel, no click-catcher.
  const shown = group ?? lastGroup;
  if (!shown) return null;

  const managed = shown.kind === "agent" || shown.kind === "skill";
  const anyDisabled = shown.installations.some((i) => i.enabled === false);

  // groupItems only ever creates a group from at least one observed
  // installation, so the first one stands in for the item as a whole.
  const primary = shown.installations[0];
  if (!primary) return null;

  const Icon = kindIcon(shown.kind);
  const displayName =
    shown.kind === "hook" ? hookDisplayName(shown.name) : shown.name;

  const openInFileBrowser = () => {
    void commands.revealPath(primary.path).then((response) => {
      if (response.status === "error") {
        showError({ title: FILE_BROWSER_ERROR_TITLE, message: response.error });
      }
    });
  };

  const openInEditor = () => {
    void commands
      .openInEditor(editorOpenPath(primary.path))
      .then((response) => {
        if (response.status === "error") {
          showError({
            title: EDITOR_ERROR_TITLE,
            message: response.error,
            steps: EDITOR_ERROR_STEPS,
          });
        }
      });
  };

  return (
    <>
      {/* Sits below the flyout (which absorbs its own clicks) and above the
          table — a click anywhere else in the content area closes the
          panel. Transparent rather than a dark scrim: the table stays
          fully readable while the flyout is open. Stops short of the
          sidebar and status footer so both stay live. */}
      {open ? (
        // biome-ignore lint/a11y/noStaticElementInteractions: transparent click-catcher, not a control
        // biome-ignore lint/a11y/useKeyWithClickEvents: Escape already closes via the keydown listener above
        <div
          className="fixed inset-y-0 right-0 left-56 z-[18]"
          onClick={onClose}
        />
      ) : null}
      <aside
        className={cn(
          "fixed top-0 right-0 bottom-7 z-[19] flex w-[min(30rem,85vw)] flex-col overflow-y-auto border-l bg-background shadow-lg transition-[transform,opacity] duration-200 ease-out",
          open
            ? "translate-x-0 opacity-100"
            : "pointer-events-none translate-x-full opacity-0",
        )}
        inert={!open}
      >
        <div className="flex items-start justify-between gap-2 p-6 pb-3">
          <div className="flex min-w-0 items-center gap-2">
            <Icon className="size-5 shrink-0 text-muted-foreground" />
            <h2 className="truncate font-semibold">{displayName}</h2>
          </div>
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label="Close"
            title="Close"
            onClick={onClose}
          >
            <X className="size-4" />
          </Button>
        </div>
        <div className="space-y-6 px-6 pb-6">
          {shown.description ? (
            <p className="text-sm text-muted-foreground">{shown.description}</p>
          ) : null}

          <ItemDetailMeta group={shown} primary={primary} />

          <div className="flex flex-wrap gap-2">
            {managed ? (
              <Button
                size="sm"
                variant="outline"
                disabled={busy}
                onClick={() =>
                  void toggle(primary.scope, shown.name, anyDisabled)
                }
              >
                {anyDisabled ? "Turn on" : "Turn off"}
              </Button>
            ) : null}
            <Button
              size="sm"
              variant="outline"
              disabled={busy}
              onClick={() => setConfirmOpen(true)}
            >
              Remove…
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button size="sm" variant="outline">
                    <ExternalLink className="size-4" />
                    {OPEN_IN_LABEL}
                  </Button>
                }
              />
              <DropdownMenuContent>
                <DropdownMenuItem onClick={openInFileBrowser}>
                  {OPEN_IN_FILE_BROWSER_LABEL}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={openInEditor}>
                  {OPEN_IN_EDITOR_LABEL}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <ReportDialog
              scope={primary.scope}
              name={shown.name}
              kind={shown.kind}
            />
          </div>

          <div className="space-y-2.5">
            <SectionLabel>Content</SectionLabel>
            <ItemPreview
              scope={primary.scope}
              kind={shown.kind}
              name={shown.name}
              harness={primary.harness}
            />
          </div>
        </div>
        <ConfirmDialog
          open={confirmOpen}
          onOpenChange={setConfirmOpen}
          title={`Remove ${shown.name}?`}
          description="The files vstack manages will be moved to the trash, and it will stop being kept up to date."
          confirmLabel="Remove"
          destructive
          busy={busy}
          onConfirm={() => {
            void removeItem(primary.scope, shown.name).then(() =>
              setConfirmOpen(false),
            );
          }}
        />
      </aside>
    </>
  );
}
