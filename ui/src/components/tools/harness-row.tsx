import type { HarnessId, ItemKind } from "@/bindings";
import { KindCountBadges } from "@/components/kind-count-badges";
import { StatusDot } from "@/components/status-dot";
import { toolName } from "@/lib/labels";
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

  if (!detectedRoot) {
    return (
      <div className="flex items-center gap-3 py-2.5">
        <StatusDot tone="muted" />
        <span className="min-w-0 flex-1 truncate text-muted-foreground">
          {name}
        </span>
        <span className="text-xs text-muted-foreground">Not installed</span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-3 py-2.5">
      <StatusDot tone="good" />
      <span className="w-36 shrink-0 truncate font-semibold">{name}</span>
      <span className="hidden w-20 shrink-0 truncate font-mono text-xs text-muted-foreground sm:inline">
        {version}
      </span>
      <span className="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground">
        {detectedRoot}
      </span>
      <div className="flex shrink-0 flex-wrap justify-end gap-1.5">
        <KindCountBadges
          counts={counts}
          onKindClick={(kind) => goToLibrary({ tool: id, kind })}
        />
      </div>
    </div>
  );
}
