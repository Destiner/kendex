import { HarnessList } from "@/components/harnesses/harness-list";
import { PageHeader } from "@/components/page-header";

/** The AI coding tools on this machine, and where each keeps its files. */
export function HarnessesPage() {
  return (
    <div>
      <PageHeader
        title="Harnesses"
        subtitle="The AI coding tools kendex writes to on this machine"
      />
      <HarnessList />
    </div>
  );
}
