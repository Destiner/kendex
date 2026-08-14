import type { ItemKind } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { kindLabel } from "@/lib/labels";

/** A compact scope summary: title, one-line description, linked count pills. */
export function ScopeCard({
  title,
  subtitle,
  counts,
  onKindClick,
}: {
  title: string;
  subtitle?: string;
  counts: [ItemKind, number][];
  onKindClick?: (kind: ItemKind) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        {subtitle ? <CardDescription>{subtitle}</CardDescription> : null}
      </CardHeader>
      <CardContent>
        <div className="flex flex-wrap gap-1.5">
          {counts.length === 0 ? (
            <span className="text-sm text-muted-foreground">
              Nothing from vstack yet.
            </span>
          ) : (
            counts.map(([kind, count]) =>
              onKindClick ? (
                <Badge
                  key={kind}
                  variant="outline"
                  className="cursor-pointer hover:bg-accent"
                  render={
                    <button type="button" onClick={() => onKindClick(kind)}>
                      {count} {kindLabel(kind, count)}
                    </button>
                  }
                />
              ) : (
                <Badge key={kind} variant="outline">
                  {count} {kindLabel(kind, count)}
                </Badge>
              ),
            )
          )}
        </div>
      </CardContent>
    </Card>
  );
}
