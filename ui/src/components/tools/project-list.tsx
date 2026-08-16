import { Trash2 } from "lucide-react";
import { useState } from "react";
import type { ItemKind } from "@/bindings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { KindCountBadges } from "@/components/kind-count-badges";
import { AddProjectCard } from "@/components/tools/add-project-card";
import { ScopeCard } from "@/components/tools/scope-card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { countByKind } from "@/lib/derive";
import { CONTENT_WIDTH, PAGE_BODY } from "@/lib/layout";
import { cn } from "@/lib/utils";
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
          <span className="truncate text-sm font-medium text-foreground">
            {name}
          </span>
          {missing ? (
            <Badge variant="destructive">Folder not found</Badge>
          ) : null}
        </div>
        <p className="truncate font-mono text-xs text-muted-foreground">
          {root}
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-1.5">
        <KindCountBadges
          counts={counts}
          onKindClick={(kind) => {
            setScope({ project: root });
            goToLibrary({ kind });
          }}
        />
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
  const [removeTarget, setRemoveTarget] = useState<string | null>(null);

  const globalItems =
    result?.items.filter((i) => i.scope.scope === "global") ?? [];
  const projects = settings?.projects ?? [];

  return (
    <div className={PAGE_BODY}>
      <div className={cn("flex flex-col gap-10", CONTENT_WIDTH)}>
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
          <p className="py-3.5 text-sm text-muted-foreground">
            No projects yet — add one below to manage its tools.
          </p>
        ) : (
          <div className="flex flex-col">
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

        <AddProjectCard
          projects={projects}
          registerProject={registerProject}
          discoverProjects={discoverProjects}
        />

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
