import { InstructionBox } from "@/components/customize/instruction-box";
import {
  LAUNCH_LABEL,
  SHARED_ADDITIONAL_HELP,
  SHARED_ADDITIONAL_LABEL,
  SHARED_LAUNCH_HELP,
  SHARED_SKILL_HELP,
  SHARED_SKILL_LABEL,
} from "@/lib/copy-customize";
import type { SharedCustomization } from "@/lib/customization";
import {
  type Draft,
  type InstructionTable,
  SHARED_KEY,
  setInstruction,
} from "@/lib/editor-draft";

/** The `all` row of each instruction table: what every agent or skill here
 *  gets, on top of whatever its own page sets. */
export function SharedInstructions({
  shared,
  onChange,
}: {
  shared: SharedCustomization;
  onChange: (change: (draft: Draft) => Draft) => void;
}) {
  const box = (
    label: string,
    help: string,
    table: InstructionTable,
    value: string | null,
  ) => (
    <InstructionBox
      label={label}
      help={help}
      value={value}
      shared={null}
      onChange={(text) =>
        onChange((draft) => setInstruction(draft, table, SHARED_KEY, text))
      }
    />
  );

  return (
    <div className="flex flex-col gap-6 pt-1">
      {box(
        LAUNCH_LABEL,
        SHARED_LAUNCH_HELP,
        "agent-launch-instructions",
        shared.launch,
      )}
      {box(
        SHARED_ADDITIONAL_LABEL,
        SHARED_ADDITIONAL_HELP,
        "agent-additional-instructions",
        shared.additional,
      )}
      {box(
        SHARED_SKILL_LABEL,
        SHARED_SKILL_HELP,
        "skill-instructions",
        shared.instructions,
      )}
    </div>
  );
}
