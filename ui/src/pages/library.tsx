import { AddCatalogView } from "@/components/library/add-catalog-view";
import { InstalledView } from "@/components/library/installed-view";
import { PageHeader } from "@/components/page-header";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { type LibraryTab, useNavStore } from "@/stores/nav";

export function LibraryPage() {
  const libraryTab = useNavStore((s) => s.libraryTab);
  const goToLibrary = useNavStore((s) => s.goToLibrary);

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Library"
        subtitle="What's installed, and where to add more"
        wide
      />
      <Tabs
        value={libraryTab}
        onValueChange={(value) => goToLibrary({ tab: value as LibraryTab })}
        className="flex min-h-0 flex-1 flex-col gap-0"
      >
        <div className={cn("py-3", PAGE_GUTTER)}>
          <div className={WIDE_CONTENT_WIDTH}>
            <TabsList>
              <TabsTrigger value="installed">Installed</TabsTrigger>
              <TabsTrigger value="add">Add from a catalog</TabsTrigger>
            </TabsList>
          </div>
        </div>
        <TabsContent value="installed" className="flex min-h-0 flex-1 flex-col">
          <InstalledView />
        </TabsContent>
        <TabsContent value="add" className="min-h-0 flex-1 overflow-y-auto">
          <AddCatalogView />
        </TabsContent>
      </Tabs>
    </div>
  );
}
