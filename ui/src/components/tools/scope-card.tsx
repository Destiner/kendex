import type { ItemKind } from "@/bindings";
import { KindCountBadges } from "@/components/kind-count-badges";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

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
          <KindCountBadges
            counts={counts}
            onKindClick={onKindClick}
            emptyLabel="Nothing from vstack yet."
            emptyClassName="text-sm text-muted-foreground"
          />
        </div>
      </CardContent>
    </Card>
  );
}
