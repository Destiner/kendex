import { X } from "lucide-react";
import type { EditorInventory } from "@/bindings";
import { AddEntry } from "@/components/editor/controls";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  type Draft,
  type InstructionTable,
  orderedKeys,
  SHARED_KEY,
  setInstruction,
} from "@/lib/editor-draft";

type Change = (change: (draft: Draft) => Draft) => void;

export function InstructionsTab({
  draft,
  inventory,
  onChange,
}: {
  draft: Draft;
  inventory: EditorInventory | null;
  onChange: Change;
}) {
  const agents = inventory?.declaredAgents ?? [];
  const skills = [
    ...new Set([
      ...(inventory?.declaredSkills ?? []),
      ...(inventory?.availableSkills ?? []),
    ]),
  ].sort();

  return (
    <div className="space-y-8">
      <Section
        title="Launch instructions"
        caption="Added at the start of every agent file vstack writes."
        table="agent-launch-instructions"
        candidates={agents}
        draft={draft}
        onChange={onChange}
      />
      <Section
        title="Additional instructions"
        caption="Added at the end of every agent file vstack writes."
        table="agent-additional-instructions"
        candidates={agents}
        draft={draft}
        onChange={onChange}
      />
      <Section
        title="Skill instructions"
        caption="Added to a skill's own instructions — the author's text is never overwritten."
        table="skill-instructions"
        candidates={skills}
        draft={draft}
        onChange={onChange}
      />
    </div>
  );
}

function Section({
  title,
  caption,
  table,
  candidates,
  draft,
  onChange,
}: {
  title: string;
  caption: string;
  table: InstructionTable;
  candidates: string[];
  draft: Draft;
  onChange: Change;
}) {
  const entries = draft[table] ?? {};
  const keys = orderedKeys(Object.keys(entries));
  const options = [SHARED_KEY, ...candidates].filter(
    (key) => !(key in entries),
  );

  return (
    <section className="space-y-3">
      <div>
        <h2 className="text-sm font-medium">{title}</h2>
        <p className="text-xs text-muted-foreground">{caption}</p>
      </div>
      {keys.length === 0 ? (
        <p className="text-sm text-muted-foreground">Nothing set here yet.</p>
      ) : null}
      {keys.map((key) => (
        <div key={key} className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">{key}</span>
            {key === SHARED_KEY ? (
              <span className="text-xs text-muted-foreground">
                Applies to every entry, shown first
              </span>
            ) : null}
            <Button
              variant="ghost"
              size="icon"
              aria-label={`Remove ${key}`}
              className="ml-auto"
              onClick={() =>
                onChange((current) => setInstruction(current, table, key, null))
              }
            >
              <X className="size-4" />
            </Button>
          </div>
          <Textarea
            aria-label={`${title} for ${key}`}
            value={entries[key]}
            onChange={(event) =>
              onChange((current) =>
                setInstruction(current, table, key, event.target.value),
              )
            }
          />
        </div>
      ))}
      <AddEntry
        placeholder="Add instructions for…"
        options={options}
        onAdd={(key) =>
          onChange((current) => setInstruction(current, table, key, ""))
        }
      />
    </section>
  );
}
