import { PageHeader } from "@/components/page-header";
import { HarnessList } from "@/components/tools/harness-list";
import { ProjectList } from "@/components/tools/project-list";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
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
        <div className="border-b px-8 py-3">
          <TabsList>
            <TabsTrigger value="tools">Tools</TabsTrigger>
            <TabsTrigger value="projects">Projects</TabsTrigger>
          </TabsList>
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
