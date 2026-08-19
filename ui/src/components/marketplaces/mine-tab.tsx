import { Hammer } from "lucide-react";
import { EmptyState } from "@/components/empty-state";

/** Marketplaces the user authors. The create / use-a-folder / import flows
 * arrive with authoring — the tab states the outcome they'll unlock rather
 * than showing buttons that cannot act yet. */
export function MineTab() {
  return (
    <EmptyState icon={Hammer} title="Nothing you publish yet">
      Soon you'll build a marketplace from skills and agents you already have,
      or start an empty one and add to it later.
    </EmptyState>
  );
}
