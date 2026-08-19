import { Globe } from "lucide-react";
import { EmptyState } from "@/components/empty-state";

/** The kendex.ai directory and Skills.sh browsing arrive with the platform;
 * until those routes exist this tab says so instead of pretending an empty
 * list is an answer. Subscribing by URL already works today. */
export function CommunityTab() {
  return (
    <EmptyState icon={Globe} title="The community directory is coming">
      Browse and subscribe to community marketplaces here once kendex.ai
      launches — until then, paste any repository into Subscribe.
    </EmptyState>
  );
}
