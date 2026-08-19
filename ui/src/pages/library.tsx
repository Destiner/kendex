import { InstalledView } from "@/components/library/installed-view";
import { PageHeader } from "@/components/page-header";

// A base page with no tabs: the installed table and the "Not managed yet"
// section. Browsing and subscribing live on the Marketplaces page.
export function LibraryPage() {
  return (
    <div className="flex h-full flex-col">
      <PageHeader title="My Library" wide />
      <InstalledView />
    </div>
  );
}
