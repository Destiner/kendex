import { UnmanagedItems } from "@/components/unmanaged-items";
import { mergeDriftRows } from "@/lib/drift-merge";
import { scopeName } from "@/lib/labels";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";

/**
 * Items on this machine that vstack was never asked to look after, with the
 * offer to take them on. This lives on the Library's Installed tab because
 * that is where a person looks at what is on the machine; the Review page
 * is for what needs deciding or applying, and adopting is neither — it is
 * an offer, taken up when the person wants it. Follows the app-wide scope
 * like everything else on the tab.
 */
export function NotManagedPanel() {
  const views = useAuditStore((s) => s.views);
  const busy = useAuditStore((s) => s.busy);
  const adopt = useAuditStore((s) => s.adopt);
  const scope = useNavStore((s) => s.scope);
  const perScope = views
    .filter((view) => {
      if (scope === "all") return true;
      if (scope === "global") return view.scope.scope === "global";
      return (
        view.scope.scope === "project" && view.scope.root === scope.project
      );
    })
    .map((view) => ({
      view,
      rows: mergeDriftRows(
        view.drift.filter((row) => row.state === "unmanaged"),
      ),
    }))
    .filter(({ rows }) => rows.length > 0);
  if (perScope.length === 0) return null;
  const several = perScope.length > 1;
  return (
    <div className="flex flex-col gap-6 pb-8">
      {perScope.map(({ view, rows }) => (
        <UnmanagedItems
          key={scopeName(view.scope)}
          rows={rows}
          busy={busy}
          title={
            several ? `Not managed yet — ${scopeName(view.scope)}` : undefined
          }
          onAdopt={(kind, name, harness, opts) =>
            adopt(view.scope, kind, name, harness, opts)
          }
        />
      ))}
    </div>
  );
}
