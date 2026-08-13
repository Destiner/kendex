import { CheckCircle2 } from "lucide-react";
import { useEffect } from "react";
import { PageHeader } from "@/components/page-header";
import { SyncScopeCard } from "@/components/sync-scope";
import { scopeLabel } from "@/lib/derive";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";

export function AuditPage() {
  const { views, auditing, error, busy, refresh, applyPlan, adopt } =
    useAuditStore();
  const scope = useNavStore((s) => s.scope);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const visible = views.filter((view) => {
    if (scope === "all") return true;
    if (scope === "global") return view.scope.scope === "global";
    return view.scope.scope === "project" && view.scope.root === scope.project;
  });
  const active = visible.filter(
    (view) =>
      view.drift.length > 0 ||
      view.notes.length > 0 ||
      view.warnings.length > 0,
  );
  const allClean = !auditing && active.length === 0;

  return (
    <div>
      <PageHeader
        title="Sync"
        subtitle="Review what's out of sync, then apply fixes"
      />
      <div className="space-y-4 p-8">
        {error ? <p className="text-sm text-destructive">{error}</p> : null}
        {auditing && views.length === 0 ? (
          <p className="text-sm text-muted-foreground">Checking for changes…</p>
        ) : null}
        {allClean ? (
          <div className="flex flex-col items-center gap-2 py-16 text-center">
            <CheckCircle2 className="size-8 text-muted-foreground" />
            <p className="font-medium">Everything is in sync.</p>
            <p className="text-sm text-muted-foreground">
              Changes from Customize or your catalogs will show up here.
            </p>
          </div>
        ) : (
          active.map((view) => (
            <SyncScopeCard
              key={scopeLabel(view.scope)}
              view={view}
              busy={busy}
              onApply={(removeOrphans) =>
                void applyPlan(view.scope, removeOrphans)
              }
              onAdopt={(kind, name, harness) =>
                void adopt(view.scope, kind, name, harness)
              }
            />
          ))
        )}
      </div>
    </div>
  );
}
