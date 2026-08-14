import { Trash2 } from "lucide-react";
import type { ItemKind } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { kindLabel } from "@/lib/labels";

export function ScopeCard({
  title,
  path,
  counts,
  missing,
  subtitle,
  onRemove,
}: {
  title: string;
  path?: string;
  counts: [ItemKind, number][];
  missing?: boolean;
  subtitle?: string;
  onRemove?: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-start gap-2 text-base">
          <div className="min-w-0">
            <span className="break-all font-semibold">{title}</span>
            {path ? (
              <p className="truncate font-mono text-xs font-normal text-muted-foreground">
                {path}
              </p>
            ) : null}
          </div>
          {missing ? (
            <Badge variant="destructive">Folder not found</Badge>
          ) : null}
          {onRemove ? (
            <Button
              variant="ghost"
              size="icon"
              className="ml-auto shrink-0"
              aria-label={`Stop tracking ${title}`}
              onClick={onRemove}
            >
              <Trash2 className="size-4" />
            </Button>
          ) : null}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {subtitle ? (
          <p className="text-sm text-muted-foreground">{subtitle}</p>
        ) : null}
        {missing ? (
          <p className="text-sm text-muted-foreground">
            This folder could not be found on this machine.
          </p>
        ) : null}
        <div className="flex flex-wrap gap-1.5">
          {counts.length === 0 ? (
            <span className="text-sm text-muted-foreground">
              Nothing from vstack yet.
            </span>
          ) : (
            counts.map(([kind, count]) => (
              <Badge key={kind} variant="outline">
                {count} {kindLabel(kind, count)}
              </Badge>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
