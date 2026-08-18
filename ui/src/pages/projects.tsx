import { PageHeader } from "@/components/page-header";
import { ProjectList } from "@/components/tools/project-list";

/** The repositories vstack keeps in sync, and the way to add or drop one. */
export function ProjectsPage() {
  return (
    <div>
      <PageHeader
        title="Projects"
        subtitle="Repositories vstack keeps in sync, alongside your personal setup"
      />
      <ProjectList />
    </div>
  );
}
