import { X } from "lucide-react";
import type { EditorInventory } from "@/bindings";
import { AddEntry } from "@/components/customize/controls";
import { Button } from "@/components/ui/button";
import {
  SKILLS_AUTOMATIC,
  SKILLS_BACK_TO_AUTOMATIC,
  SKILLS_CHOSEN,
  SKILLS_NONE_AVAILABLE,
} from "@/lib/copy-customize";
import {
  clearAgentSkills,
  type Draft,
  setAgentSkill,
} from "@/lib/editor-draft";

/**
 * Which skills one agent gets. Chosen skills are chips and the rest live
 * behind a picker: an agent has a handful, a machine has dozens, and a wall
 * of unticked boxes hides the answer to "what does this agent have".
 */
export function ItemSkills({
  agent,
  chosen,
  inventory,
  onChange,
}: {
  agent: string;
  /** null while vstack picks skills from the agent's tags. */
  chosen: string[] | null;
  inventory: EditorInventory | null;
  onChange: (change: (draft: Draft) => Draft) => void;
}) {
  const known = [
    ...new Set([
      ...(inventory?.declaredSkills ?? []),
      ...(inventory?.availableSkills ?? []),
      ...(chosen ?? []),
    ]),
  ].sort();
  const unchosen = known.filter((skill) => !(chosen ?? []).includes(skill));

  return (
    <div className="flex flex-col gap-3">
      <p className="text-[13px] text-muted-foreground">
        {chosen ? SKILLS_CHOSEN : SKILLS_AUTOMATIC}
      </p>
      {chosen && chosen.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {chosen.map((skill) => (
            <span
              key={skill}
              className="inline-flex h-7 items-center gap-1 rounded-full bg-secondary pr-1.5 pl-3 text-xs font-medium"
            >
              {skill}
              <button
                type="button"
                aria-label={`Remove ${skill}`}
                className="rounded-full p-0.5 text-muted-foreground hover:text-foreground"
                onClick={() =>
                  onChange((draft) => setAgentSkill(draft, agent, skill, false))
                }
              >
                <X className="size-3.5" />
              </button>
            </span>
          ))}
        </div>
      ) : null}
      <div className="flex flex-wrap items-center gap-2">
        {unchosen.length > 0 ? (
          <AddEntry
            placeholder="Add a skill…"
            options={unchosen}
            onAdd={(skill) =>
              onChange((draft) => setAgentSkill(draft, agent, skill, true))
            }
          />
        ) : (
          <p className="text-[13px] text-muted-foreground">
            {SKILLS_NONE_AVAILABLE}
          </p>
        )}
        {chosen ? (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onChange((draft) => clearAgentSkills(draft, agent))}
          >
            {SKILLS_BACK_TO_AUTOMATIC}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
