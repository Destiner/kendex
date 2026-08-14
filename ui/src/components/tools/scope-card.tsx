import type { ItemKind } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { KindCountBadges } from "@/components/kind-count-badges";
import { Card, CardContent, CardHeader } from "@/components/ui/card";

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
    <Card className="gap-3 py-4">
      <CardHeader className="gap-1">
        <SectionLabel>{title}</SectionLabel>
        {subtitle ? (
          <p className="text-xs text-muted-foreground">{subtitle}</p>
        ) : null}
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
