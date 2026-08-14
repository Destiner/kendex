import type { HarnessId } from "@/bindings";
import type { RecentGroup } from "@/lib/derive";
import { kindIcon } from "@/lib/kind-icon";
import {
  hookDisplayName,
  kindLabel,
  RECENT_ACTIVITY_EMPTY,
  toolName,
} from "@/lib/labels";
import { relativeTime } from "@/lib/relative-time";
import { useNavStore } from "@/stores/nav";

/** What changed on this machine lately — the one thing Home can say that
 *  the status footer's freshness/pending counts don't. */
export function RecentActivity({ groups }: { groups: RecentGroup[] }) {
  const goToLibrary = useNavStore((s) => s.goToLibrary);

  if (groups.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">{RECENT_ACTIVITY_EMPTY}</p>
    );
  }

  return (
    <div className="divide-y rounded-lg border px-4">
      {groups.map((group) => {
        const Icon = kindIcon(group.kind);
        const name =
          group.kind === "hook" ? hookDisplayName(group.name) : group.name;
        const tools = group.harnesses
          .map((h) => toolName(h as HarnessId))
          .join(", ");
        return (
          <button
            key={group.key}
            type="button"
            className="-mx-4 flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-accent"
            onClick={() => goToLibrary({ kind: group.kind })}
          >
            <Icon className="size-4 shrink-0 text-muted-foreground" />
            <span className="min-w-0 flex-1 truncate font-medium">{name}</span>
            <span className="hidden shrink-0 truncate text-xs text-muted-foreground sm:inline">
              {kindLabel(group.kind)} · {tools}
            </span>
            <span className="w-16 shrink-0 text-right text-xs text-muted-foreground">
              {relativeTime(group.modifiedAt * 1000, Date.now())}
            </span>
          </button>
        );
      })}
    </div>
  );
}
