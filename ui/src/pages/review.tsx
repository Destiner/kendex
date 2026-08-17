import { CheckCircle2 } from "lucide-react";
import { useEffect } from "react";
import { PageHeader } from "@/components/page-header";
import { ScopeErrorCard } from "@/components/scope-error-card";
import { StatusNote } from "@/components/status-note";
import { SyncScopeCard } from "@/components/sync-scope";
import { blockedCount, openCount } from "@/lib/audit-counts";
import { REVIEW_SUBTITLE } from "@/lib/copy";
import { scopeLabel } from "@/lib/derive";
import { CONTENT_WIDTH, PAGE_BODY } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";

export function ReviewPage() {
  const { views, auditing, error, busy, refresh, applyPlan, adopt, dismiss } =
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
  // A scope is finished when nothing in it waits on a person: no change to
  // apply, no note, and no decision left to make. Findings someone already
  // ruled on do not keep the page open — that is what ruling on them is for.
  const active = visible.filter(
    (view) =>
      view.error != null ||
      view.drift.length > 0 ||
      view.notes.length > 0 ||
      view.warnings.length > 0 ||
      blockedCount(view) > 0 ||
      openCount(view) > 0,
  );
  const allClean = !auditing && active.length === 0;

  return (
    <div>
      <PageHeader title="Review & apply" subtitle={REVIEW_SUBTITLE} />
      <div className={PAGE_BODY}>
        <div className={cn("flex flex-col gap-12", CONTENT_WIDTH)}>
          {error ? (
            <StatusNote tone="critical" title="Checking for changes failed">
              {error}
            </StatusNote>
          ) : null}
          {auditing && views.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              Checking for changes…
            </p>
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
            active.map((view) =>
              view.error ? (
                <ScopeErrorCard
                  key={scopeLabel(view.scope)}
                  view={view}
                  error={view.error}
                />
              ) : (
                <SyncScopeCard
                  key={scopeLabel(view.scope)}
                  view={view}
                  busy={busy}
                  onApply={(removeOrphans, allowUnsafe) =>
                    void applyPlan(view.scope, removeOrphans, allowUnsafe)
                  }
                  onAdopt={(kind, name, harness, opts) =>
                    void adopt(view.scope, kind, name, harness, opts)
                  }
                  onDismiss={(tokens, reason) =>
                    void dismiss(view.scope, tokens, reason)
                  }
                />
              ),
            )
          )}
        </div>
      </div>
    </div>
  );
}
