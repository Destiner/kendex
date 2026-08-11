import type { EditorInventory } from "@/bindings";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  agentRows,
  type Draft,
  setAgentSkill,
  skillColumns,
} from "@/lib/editor-draft";

export function AgentSkillsTab({
  draft,
  inventory,
  onChange,
}: {
  draft: Draft;
  inventory: EditorInventory | null;
  onChange: (change: (draft: Draft) => Draft) => void;
}) {
  const rows = agentRows(draft, inventory?.declaredAgents ?? []);
  const columns = skillColumns(draft, [
    ...(inventory?.declaredSkills ?? []),
    ...(inventory?.availableSkills ?? []),
  ]);
  const assigned = draft["agent-skills"] ?? {};

  if (rows.length === 0 || columns.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {rows.length === 0
          ? "No agents here yet — install an agent before assigning skills."
          : "No skills available from the catalogs added here."}
      </p>
    );
  }

  return (
    <div className="space-y-3">
      <p className="text-sm text-muted-foreground">
        A row here overrides the automatic skill assignment; removals stay
        removed.
      </p>
      <div className="overflow-x-auto rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="sticky left-0 bg-background">
                Agent
              </TableHead>
              {columns.map((skill) => (
                <TableHead key={skill} className="whitespace-nowrap">
                  {skill}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((agent) => (
              <TableRow key={agent}>
                <TableCell className="sticky left-0 bg-background font-medium">
                  {agent}
                  {agent in assigned ? null : (
                    <span className="ml-2 text-xs text-muted-foreground">
                      Automatic
                    </span>
                  )}
                </TableCell>
                {columns.map((skill) => (
                  <TableCell key={skill}>
                    <Checkbox
                      aria-label={`${agent} × ${skill}`}
                      checked={(assigned[agent] ?? []).includes(skill)}
                      onCheckedChange={(checked) =>
                        onChange((current) =>
                          setAgentSkill(
                            current,
                            agent,
                            skill,
                            checked === true,
                          ),
                        )
                      }
                    />
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
