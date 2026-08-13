import { Package } from "lucide-react";
import type { BundleRow } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { bundleSummary } from "@/lib/derive";

/** The ready-made sets one catalog offers, each installable as a unit. */
export function CatalogBundles({
  rows,
  busy,
  onInstall,
}: {
  rows: BundleRow[];
  busy: boolean;
  onInstall: (name: string) => void;
}) {
  if (rows.length === 0) return null;
  return (
    <div className="ml-7 space-y-2 border-l pl-3">
      {rows.map((row) => (
        <div
          key={row.name}
          className="flex items-start justify-between gap-3 text-sm"
        >
          <div className="min-w-0">
            <span className="flex flex-wrap items-center gap-2">
              <Package className="size-4 shrink-0 text-muted-foreground" />
              <span className="font-medium">{row.name}</span>
              {row.version ? (
                <Badge variant="outline">{row.version}</Badge>
              ) : null}
              {row.description ? (
                <span className="text-muted-foreground">{row.description}</span>
              ) : null}
            </span>
            <p className="truncate pl-6 text-xs text-muted-foreground">
              {bundleSummary(row.members)}
            </p>
          </div>
          {row.installed ? (
            <Badge variant="secondary" className="shrink-0">
              Installed
            </Badge>
          ) : (
            <Button
              variant="outline"
              size="sm"
              className="shrink-0"
              disabled={busy}
              onClick={() => onInstall(row.name)}
            >
              Install bundle
            </Button>
          )}
        </div>
      ))}
    </div>
  );
}
