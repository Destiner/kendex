import { PageHeader } from "@/components/page-header";
import { HarnessList } from "@/components/tools/harness-list";
import { ProjectList } from "@/components/tools/project-list";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CONTENT_WIDTH, PAGE_GUTTER } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { type ToolsTab, useNavStore } from "@/stores/nav";

export function ToolsProjectsPage() {
  const toolsTab = useNavStore((s) => s.toolsTab);
  const goToTools = useNavStore((s) => s.goToTools);

  return (
    <div>
      <PageHeader
        title="Tools & Projects"
        subtitle="Where your setup applies"
      />
      <Tabs
        value={toolsTab}
        onValueChange={(value) => goToTools(value as ToolsTab)}
      >
        <div className={cn("py-3", PAGE_GUTTER)}>
          <div className={CONTENT_WIDTH}>
            <TabsList>
              <TabsTrigger value="tools">Tools</TabsTrigger>
              <TabsTrigger value="projects">Projects</TabsTrigger>
            </TabsList>
          </div>
        </div>
        <TabsContent value="tools">
          <HarnessList />
        </TabsContent>
        <TabsContent value="projects">
          <ProjectList />
        </TabsContent>
      </Tabs>
    </div>
  );
}
