import type { HarnessId, ItemKind } from "@/bindings";
import { KindCountBadges } from "@/components/kind-count-badges";
import { ToolIcon } from "@/components/tool-icon";
import { toolName } from "@/lib/labels";
import { cn } from "@/lib/utils";
import { useNavStore } from "@/stores/nav";

/** One detected (or missing) AI coding tool, as a single compact row. */
export function HarnessRow({
  id,
  detectedRoot,
  version,
  counts,
}: {
  id: HarnessId;
  detectedRoot: string | null;
  version: string | null;
  counts: [ItemKind, number][];
}) {
  const goToLibrary = useNavStore((s) => s.goToLibrary);
  const name = toolName(id);

  // Name, then what the machine knows about it, then where it lives —
  // the same three-step read as every other row in the app: what it is,
  // one line about it, and the controls opposite.
  return (
    <div className="flex items-start justify-between gap-6 py-3.5">
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
        <p
          className={cn(
            "truncate pl-7 text-[13px] text-muted-foreground",
            detectedRoot && "font-mono",
          )}
        >
          {detectedRoot ?? "Not installed"}
        </p>
      </div>
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
