import { Package } from "lucide-react";
import type { BundleRow, Scope } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { bundleSummary } from "@/lib/derive";
import { scopeName } from "@/lib/labels";

/** One scope's ready-made sets, flattened across every catalog it uses. */
export function BundleGallery({
  scope,
  rows,
  busy,
  onInstall,
}: {
  scope: Scope;
  rows: BundleRow[];
  busy: boolean;
  onInstall: (source: string, name: string) => void;
}) {
  if (rows.length === 0) return null;
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">{scopeName(scope)}</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-3 sm:grid-cols-2">
        {rows.map((row) => (
          <div
            key={`${row.source}:${row.name}`}
            className="flex flex-col gap-2 rounded-md border p-3 text-sm"
          >
            <div className="flex flex-wrap items-center gap-2">
              <Package className="size-4 shrink-0 text-muted-foreground" />
              <span className="font-medium">{row.name}</span>
              {row.version ? (
                <Badge variant="outline" className="font-mono">
                  {row.version}
                </Badge>
              ) : null}
              <Badge variant="secondary">{row.source}</Badge>
            </div>
            {row.description ? (
              <p className="text-muted-foreground">{row.description}</p>
            ) : null}
            <p className="truncate text-xs text-muted-foreground">
              {bundleSummary(row.members)}
            </p>
            {row.installed ? (
              <Badge variant="secondary" className="w-fit">
                Installed
              </Badge>
            ) : (
              <Button
                variant="outline"
                size="sm"
                className="w-fit"
                disabled={busy}
                onClick={() => onInstall(row.source, row.name)}
              >
                Install bundle
              </Button>
            )}
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
