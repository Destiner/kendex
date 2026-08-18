import { Bot } from "lucide-react";
import { useState } from "react";
import type { EditorInventory, HarnessId } from "@/bindings";
import { FrontmatterFields } from "@/components/editor/frontmatter-fields";
import { EmptyState } from "@/components/empty-state";
import { Pill } from "@/components/pill";
import { StatusLine } from "@/components/status-note";
import { ToolIcon } from "@/components/tool-icon";
import {
  FRONTMATTER_HELP,
  FRONTMATTER_IGNORED,
  NO_AGENTS_YET,
  NO_AGENTS_YET_BODY,
} from "@/lib/copy-customize";
import {
  type Draft,
  EMPTY_FRONTMATTER,
  setFrontmatterField,
} from "@/lib/editor-draft";
import { toolName } from "@/lib/labels";

/** Cursor renders rules, not agent files — its frontmatter is never read. */
const IGNORED_BY = new Set<HarnessId>(["cursor"]);

/**
 * Per-agent settings, one tool at a time. The tool is a row of pills rather
 * than a second tab bar: two stacked tab bars leave a reader unsure which
 * one they just changed.
 */
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
  const [selected, setSelected] = useState<HarnessId | null>(null);
  const harness = selected ?? harnesses[0];
  if (!harness) return null;

  const perAgent = draft["agent-frontmatter"]?.[harness] ?? {};
  const agents = [
    ...new Set([
      ...(inventory?.declaredAgents ?? []),
      ...Object.keys(perAgent),
    ]),
  ].sort();

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-1.5">
        {harnesses.map((id) => (
          <Pill
            key={id}
            selected={id === harness}
            onClick={() => setSelected(id)}
          >
            <ToolIcon harness={id} className="size-3.5" />
            {toolName(id)}
          </Pill>
        ))}
      </div>
      <StatusLine tone={IGNORED_BY.has(harness) ? "warning" : "info"}>
        {IGNORED_BY.has(harness)
          ? FRONTMATTER_IGNORED(toolName(harness))
          : FRONTMATTER_HELP}
      </StatusLine>
      {agents.length === 0 ? (
        <EmptyState icon={Bot} title={NO_AGENTS_YET}>
          {NO_AGENTS_YET_BODY}
        </EmptyState>
      ) : (
        <div className="flex flex-col divide-y rounded-lg border">
          {agents.map((agent) => (
            <div key={agent} className="flex flex-col gap-3 px-4 py-4">
              <p className="text-sm font-medium">{agent}</p>
              <FrontmatterFields
                overrides={perAgent[agent] ?? EMPTY_FRONTMATTER}
                onSet={(field, value) =>
                  onChange((current) =>
                    setFrontmatterField(current, harness, agent, field, value),
                  )
                }
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
