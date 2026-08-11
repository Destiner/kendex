import type { EditorInventory } from "@/bindings";
import { FrontmatterFields } from "@/components/editor/frontmatter-fields";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  type Draft,
  EMPTY_FRONTMATTER,
  setFrontmatterField,
} from "@/lib/editor-draft";
import { toolName } from "@/lib/labels";

/** Cursor renders rules, not agent files — its frontmatter is never read. */
const IGNORED_BY = new Set(["cursor"]);

export function FrontmatterTab({
  draft,
  inventory,
  onChange,
}: {
  draft: Draft;
  inventory: EditorInventory | null;
  onChange: (change: (draft: Draft) => Draft) => void;
}) {
  const harnesses = inventory?.harnesses ?? [];
  if (harnesses.length === 0) return null;

  return (
    <Tabs defaultValue={harnesses[0]}>
      <TabsList variant="line">
        {harnesses.map((harness) => (
          <TabsTrigger key={harness} value={harness}>
            {toolName(harness)}
          </TabsTrigger>
        ))}
      </TabsList>
      {harnesses.map((harness) => (
        <TabsContent key={harness} value={harness} className="space-y-4 pt-2">
          <p className="text-xs text-muted-foreground">
            {IGNORED_BY.has(harness)
              ? "Cursor doesn't use agent settings, so values saved here are stored but have no effect."
              : "Project values always win over the catalog's; leave a field blank to keep the catalog's value."}
          </p>
          <HarnessAgents
            draft={draft}
            harness={harness}
            declared={inventory?.declaredAgents ?? []}
            onChange={onChange}
          />
        </TabsContent>
      ))}
    </Tabs>
  );
}

function HarnessAgents({
  draft,
  harness,
  declared,
  onChange,
}: {
  draft: Draft;
  harness: string;
  declared: string[];
  onChange: (change: (draft: Draft) => Draft) => void;
}) {
  const perAgent = draft["agent-frontmatter"]?.[harness] ?? {};
  const agents = [...new Set([...declared, ...Object.keys(perAgent)])].sort();

  if (agents.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No agents here yet — install an agent before changing its settings.
      </p>
    );
  }

  return (
    <>
      {agents.map((agent) => (
        <Card key={agent}>
          <CardHeader>
            <CardTitle className="text-sm">{agent}</CardTitle>
          </CardHeader>
          <CardContent>
            <FrontmatterFields
              overrides={perAgent[agent] ?? EMPTY_FRONTMATTER}
              onSet={(field, value) =>
                onChange((current) =>
                  setFrontmatterField(current, harness, agent, field, value),
                )
              }
            />
          </CardContent>
        </Card>
      ))}
    </>
  );
}
