import type { ItemKind } from "@/bindings";
import { KindCountBadges } from "@/components/kind-count-badges";
import { Section } from "@/components/section";

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
    <Section title={title} description={subtitle}>
      <div className="flex flex-wrap gap-1.5">
        <KindCountBadges
          counts={counts}
          onKindClick={onKindClick}
          emptyLabel="Nothing from vstack yet."
          emptyClassName="text-sm text-muted-foreground"
        />
      </div>
    </Section>
  );
}
