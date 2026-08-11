import { useEffect } from "react";
import type { AuditView, DriftState } from "@/bindings";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { scopeLabel } from "@/lib/derive";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";

const STATE_STYLE: Record<
  DriftState,
  "default" | "secondary" | "destructive" | "outline"
> = {
  missing: "default",
  stale: "secondary",
  orphaned: "outline",
  unmanaged: "outline",
  conflict: "destructive",
};

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

  return (
    <div>
      <PageHeader
        title="Audit"
        subtitle="Declared vs. observed — drift and the plan to fix it"
      />
      <div className="space-y-4 p-8">
        {error ? <p className="text-sm text-destructive">{error}</p> : null}
        {auditing && views.length === 0 ? (
          <p className="text-sm text-muted-foreground">auditing…</p>
        ) : null}
        {visible.map((view) => (
          <ScopeAudit
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
        ))}
        {visible.length > 0 && visible.every((v) => v.drift.length === 0) ? (
          <p className="text-sm text-muted-foreground">
            No drift — everything matches.
          </p>
        ) : null}
      </div>
    </div>
  );
}

function ScopeAudit({
  view,
  busy,
  onApply,
  onAdopt,
}: {
  view: AuditView;
  busy: boolean;
  onApply: (removeOrphans: boolean) => void;
  onAdopt: (
    kind: AuditView["drift"][number]["kind"],
    name: string,
    harness: AuditView["drift"][number]["harness"],
  ) => void;
}) {
  if (view.drift.length === 0 && view.notes.length === 0) return null;
  const hasOrphans = view.drift.some((row) => row.state === "orphaned");
  const fixable = view.plan.length > 0;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-3 text-base">
          <span className="break-all">{scopeLabel(view.scope)}</span>
          {fixable ? (
            <span className="ml-auto flex gap-2">
              <Button size="sm" disabled={busy} onClick={() => onApply(false)}>
                Apply plan
              </Button>
              {hasOrphans ? (
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busy}
                  onClick={() => onApply(true)}
                >
                  Reconcile (remove orphans)
                </Button>
              ) : null}
            </span>
          ) : null}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {view.drift.map((row) => (
          <div
            key={`${row.kind}:${row.name}:${row.harness}`}
            className="flex items-start gap-2 text-sm"
          >
            <Badge variant={STATE_STYLE[row.state]}>{row.state}</Badge>
            <span className="font-medium">{row.name}</span>
            <span className="text-muted-foreground">
              {row.kind} · {row.harness}
            </span>
            <span className="min-w-0 flex-1 break-all text-muted-foreground">
              {row.detail}
            </span>
            {row.state === "unmanaged" ? (
              <Button
                size="sm"
                variant="outline"
                disabled={busy}
                onClick={() => onAdopt(row.kind, row.name, row.harness)}
              >
                Adopt
              </Button>
            ) : null}
          </div>
        ))}
        {view.plan.length > 0 ? (
          <div className="rounded-md border bg-muted/40 p-3">
            <p className="mb-1 text-xs font-medium text-muted-foreground">
              Plan preview
            </p>
            {view.plan.map((line) => (
              <p key={line} className="break-all text-xs text-muted-foreground">
                {line}
              </p>
            ))}
          </div>
        ) : null}
        {view.notes.map((note) => (
          <p key={note} className="text-xs text-muted-foreground">
            note: {note}
          </p>
        ))}
      </CardContent>
    </Card>
  );
}
