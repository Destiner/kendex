import type { ItemWarning } from "@/bindings";
import { Section } from "@/components/section";
import { Badge } from "@/components/ui/badge";
import { type MergedDriftRow, mergedDetail } from "@/lib/drift-merge";
import { groupWarnings } from "@/lib/group-findings";
import {
  driftDetail,
  kindLabel,
  STATE_BADGES,
  STATE_LABELS,
  toolName,
} from "@/lib/labels";

/** What applying this project would do, one line per thing it touches. */
export function ScopeChanges({ changes }: { changes: MergedDriftRow[] }) {
  if (changes.length === 0) return null;
  return (
    <Section title="Changes">
      <div className="divide-y divide-border">
        {changes.map((group) => {
          const detail = mergedDetail(group.installations.map(driftDetail));
          const tools = group.installations
            .map((row) => toolName(row.harness))
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

/** Things worth knowing that aren't a change and aren't a safety finding. */
export function ScopeNotes({
  notes,
  warnings,
}: {
  notes: string[];
  warnings: ItemWarning[];
}) {
  if (notes.length === 0 && warnings.length === 0) return null;
  return (
    <Section title="Notes">
      {notes.map((note) => (
        <p key={note} className="text-[13px] text-muted-foreground">
          {note}
        </p>
      ))}
      {groupWarnings(warnings).map((group) => (
        <p
          key={`${group.message}-${group.remediation ?? ""}`}
          className="text-[13px] text-muted-foreground"
        >
          <span className="break-all font-mono">
            {group.items.map((item) => item.name).join(", ")}
          </span>
          : {group.message}
          {group.remediation ? ` — fix: ${group.remediation}` : ""}
        </p>
      ))}
    </Section>
  );
}
