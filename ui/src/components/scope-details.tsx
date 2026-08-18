import { Section } from "@/components/section";
import { Badge } from "@/components/ui/badge";
import { type MergedDriftRow, mergedDetail } from "@/lib/drift-merge";
import {
  driftDetail,
  harnessName,
  kindLabel,
  STATE_BADGES,
  STATE_LABELS,
} from "@/lib/labels";

/** What applying this project would do, one line per thing it touches. */
export function ScopeChanges({ changes }: { changes: MergedDriftRow[] }) {
  if (changes.length === 0) return null;
  return (
    <Section title="Ready to apply">
      <div className="divide-y divide-border">
        {changes.map((group) => {
          const detail = mergedDetail(group.installations.map(driftDetail));
          const tools = group.installations
            .map((row) => harnessName(row.harness))
            .join(", ");
          return (
            <div
              key={`${group.kind}:${group.name}:${group.state}`}
              className="flex flex-wrap items-center gap-2 py-2.5 first:pt-0 last:pb-0"
            >
              <span className="text-sm font-medium">{group.name}</span>
              <Badge variant={STATE_BADGES[group.state]}>
                {STATE_LABELS[group.state]}
              </Badge>
              <span className="text-xs text-muted-foreground">
                {kindLabel(group.kind)} · {tools}
              </span>
              {detail ? (
                <span className="text-xs text-muted-foreground">{detail}</span>
              ) : null}
            </div>
          );
        })}
      </div>
    </Section>
  );
}
