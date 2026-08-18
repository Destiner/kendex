import { PageHeader } from "@/components/page-header";
import { HarnessList } from "@/components/tools/harness-list";

/** The AI coding tools on this machine, and where each keeps its files. */
export function ToolsPage() {
  return (
    <div>
      <PageHeader
        title="Tools"
        subtitle="The AI coding tools on this machine"
      />
      <HarnessList />
    </div>
  );
}
