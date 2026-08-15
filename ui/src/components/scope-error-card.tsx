import type { AuditView, ScopeError } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PROBLEM_HEADLINES, PROBLEM_STEPS } from "@/lib/error-copy";
import { scopeName, scopePath } from "@/lib/labels";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";

// A scope that couldn't be read renders this instead of its normal
// drift/safety card — an audit that never ran has nothing to report, and
// that must never look identical to a project with nothing wrong.
export function ScopeErrorCard({
  view,
  error,
}: {
  view: AuditView;
  error: ScopeError;
}) {
  const refreshScan = useScanStore((s) => s.refresh);
  const refreshAudit = useAuditStore((s) => s.refresh);
  const goTo = useNavStore((s) => s.goTo);
  const path = scopePath(view.scope);

  return (
    <Card className="border-critical/30 bg-critical/5">
      <CardHeader>
        <CardTitle className="text-base">
          <span className="break-all text-critical">
            {PROBLEM_HEADLINES[error.kind]}
          </span>
          <p className="truncate text-xs font-normal text-muted-foreground">
            {scopeName(view.scope)}
            {path ? ` · ${path}` : ""}
          </p>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <p className="break-words rounded-md bg-muted/50 p-2 font-mono text-xs text-muted-foreground">
          {error.message}
        </p>
        <ul className="list-disc space-y-1 pl-5 text-xs text-muted-foreground">
          {PROBLEM_STEPS[error.kind].map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ul>
        <div className="flex gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              void refreshScan();
              void refreshAudit();
            }}
          >
            Rescan
          </Button>
          <Button size="sm" variant="outline" onClick={() => goTo("problems")}>
            See all problems
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
