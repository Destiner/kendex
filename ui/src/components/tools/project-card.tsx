import type { ReactNode } from "react";
import type { ItemKind } from "@/bindings";
import { KindCountBadges } from "@/components/kind-count-badges";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";

/**
 * One place a setup applies — Personal, or a project folder. Personal and a
 * project are the same kind of thing to a reader, so they get the same card:
 * name and subtitle on top, counts along the bottom, actions on the right.
 * They used to differ, and the counts landing in a different place each time
 * made two identical facts look like two different ones.
 */
export function ProjectCard({
  name,
  subtitle,
  counts,
  onKindClick,
  emptyLabel,
  badge,
  action,
}: {
  name: string;
  subtitle: string;
  counts: [ItemKind, number][];
  onKindClick: (kind: ItemKind) => void;
  emptyLabel: string;
  /** A state worth flagging beside the name, e.g. a missing folder. */
  badge?: string;
  action?: ReactNode;
}) {
  return (
    <Card className="gap-3 py-4">
      <div className="flex items-start justify-between gap-3 px-4">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="truncate text-sm font-medium">{name}</span>
            {badge ? <Badge variant="destructive">{badge}</Badge> : null}
          </div>
          <p className="truncate text-[13px] text-muted-foreground">
            {subtitle}
          </p>
        </div>
        {action ? <div className="shrink-0">{action}</div> : null}
      </div>
      <div className="flex flex-wrap gap-1.5 px-4">
        <KindCountBadges
          counts={counts}
          onKindClick={onKindClick}
          emptyLabel={emptyLabel}
          emptyClassName="text-[13px] text-muted-foreground"
        />
      </div>
    </Card>
  );
}
