import type { ItemKind } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { kindLabel } from "@/lib/labels";

/** A row of item-kind count pills — clickable when a handler is given,
 *  falling back to an empty-state message when there's nothing to show. */
export function KindCountBadges({
  counts,
  onKindClick,
  emptyLabel = "Nothing yet",
  emptyClassName = "text-xs text-muted-foreground",
}: {
  counts: [ItemKind, number][];
  onKindClick?: (kind: ItemKind) => void;
  emptyLabel?: string;
  emptyClassName?: string;
}) {
  if (counts.length === 0) {
    return <span className={emptyClassName}>{emptyLabel}</span>;
  }
  return (
    <>
      {counts.map(([kind, count]) =>
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
      )}
    </>
  );
}
