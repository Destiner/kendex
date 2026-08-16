import { CheckCircle2 } from "lucide-react";
import { StatusDot } from "@/components/status-dot";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { ALL_CAUGHT_UP_DETAIL, ALL_CAUGHT_UP_TITLE } from "@/lib/copy";

export interface AttentionRow {
  key: string;
  tone: "critical" | "warning" | "info" | "muted";
  title: string;
  detail: string;
  action?: { label: string; onClick: () => void };
}

function AttentionCard({
  row,
  primary,
}: {
  row: AttentionRow;
  primary: boolean;
}) {
  return (
    <Card>
      <CardContent className="flex items-center justify-between gap-4">
        <div className="flex items-start gap-3">
          <StatusDot tone={row.tone} className="mt-2" />
          <div>
            <p className="font-medium">{row.title}</p>
            <p className="text-sm text-muted-foreground">{row.detail}</p>
          </div>
        </div>
        {row.action ? (
          <Button
            variant={primary ? "default" : "outline"}
            className="shrink-0"
            onClick={row.action.onClick}
          >
            {row.action.label}
          </Button>
        ) : null}
      </CardContent>
    </Card>
  );
}

/** The lead of Home: what needs a person's judgment, ranked by how much it
 *  blocks — or, once that list is empty, a single quiet confirmation. */
export function AttentionSection({ rows }: { rows: AttentionRow[] }) {
  if (rows.length === 0) {
    return (
      <div className="flex items-center gap-3 rounded-lg border bg-muted/30 px-5 py-4">
        <CheckCircle2 className="size-5 text-good" />
        <div>
          <p className="font-medium">{ALL_CAUGHT_UP_TITLE}</p>
          <p className="text-sm text-muted-foreground">
            {ALL_CAUGHT_UP_DETAIL}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {rows.map((row, i) => (
        <AttentionCard key={row.key} row={row} primary={i === 0} />
      ))}
    </div>
  );
}
