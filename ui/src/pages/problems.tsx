import { CheckCircle2 } from "lucide-react";
import { PageHeader } from "@/components/page-header";
import { ProblemCard } from "@/components/problem-card";
import { PROBLEMS_EMPTY, PROBLEMS_SUBTITLE } from "@/lib/error-copy";
import { useProblems } from "@/stores/problems";

export function ProblemsPage() {
  const problems = useProblems();

  return (
    <div>
      <PageHeader title="Problems" subtitle={PROBLEMS_SUBTITLE} />
      <div className="p-8">
        <div className="mx-auto w-full max-w-5xl space-y-4">
          {problems.length === 0 ? (
            <div className="flex flex-col items-center gap-2 py-16 text-center">
              <CheckCircle2 className="size-8 text-muted-foreground" />
              <p className="font-medium">{PROBLEMS_EMPTY}</p>
            </div>
          ) : (
            problems.map((problem) => (
              <ProblemCard key={problem.key} problem={problem} />
            ))
          )}
        </div>
      </div>
    </div>
  );
}
