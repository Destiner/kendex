import { useState } from "react";
import type { HarnessId } from "@/bindings";
import { FrontmatterFields } from "@/components/customize/frontmatter-fields";
import { Pill } from "@/components/pill";
import { StatusLine } from "@/components/status-note";
import { ToolIcon } from "@/components/tool-icon";
import { FRONTMATTER_HELP, FRONTMATTER_IGNORED } from "@/lib/copy-customize";
import { frontmatterFor, type ItemCustomization } from "@/lib/customization";
import { type Draft, setFrontmatterField } from "@/lib/editor-draft";
import { toolName } from "@/lib/labels";

/** Cursor renders rules, not agent files — its frontmatter is never read. */
const IGNORED_BY = new Set<HarnessId>(["cursor"]);

/**
 * One agent's settings, one tool at a time. Tools are pills rather than a
 * second tab bar: two stacked tab bars leave a reader unsure which one they
 * just changed. A pill is marked when that tool carries settings, so the
 * row answers "where have I changed something" without clicking through.
 */
export function ItemSettings({
  agent,
  customization,
  harnesses,
  onChange,
}: {
  agent: string;
  customization: ItemCustomization;
  harnesses: HarnessId[];
  onChange: (change: (draft: Draft) => Draft) => void;
}) {
  const [selected, setSelected] = useState<HarnessId | null>(null);
  const harness = selected ?? harnesses[0];
  if (!harness) return null;
  const set = new Set(customization.frontmatter.map(([id]) => id));

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
            {set.has(id) ? <Dot /> : null}
          </Pill>
        ))}
      </div>
      <StatusLine tone={IGNORED_BY.has(harness) ? "warning" : "info"}>
        {IGNORED_BY.has(harness)
          ? FRONTMATTER_IGNORED(toolName(harness))
          : FRONTMATTER_HELP}
      </StatusLine>
      <FrontmatterFields
        overrides={frontmatterFor(customization, harness)}
        onSet={(field, value) =>
          onChange((draft) =>
            setFrontmatterField(draft, harness, agent, field, value),
          )
        }
      />
    </div>
  );
}

function Dot() {
  return <span className="size-1.5 rounded-full bg-current" />;
}
