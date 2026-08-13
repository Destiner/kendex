import {
  CommitInput,
  Field,
  TriStateSelect,
} from "@/components/editor/controls";
import { Input } from "@/components/ui/input";
import {
  type DraftFrontmatter,
  formatList,
  parseList,
} from "@/lib/editor-draft";

const TEXT_FIELDS = [
  "color",
  "model",
  "effort",
  "isolation",
  "memory",
  "mode",
  "sandbox-mode",
  "model-reasoning-effort",
] as const;

const LIST_FIELDS = [
  "deny-tools",
  "allow-tools",
  "allowed-subagents",
  "nickname-candidates",
] as const;

const FLAG_FIELDS = ["pane", "background"] as const;

export type SetField = <K extends keyof DraftFrontmatter>(
  field: K,
  value: DraftFrontmatter[K],
) => void;

/** Empty means unset: a blank field is left out of the manifest entirely. */
export function FrontmatterFields({
  overrides,
  onSet,
}: {
  overrides: DraftFrontmatter;
  onSet: SetField;
}) {
  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {TEXT_FIELDS.map((field) => (
        <Field key={field} label={field}>
          <Input
            aria-label={field}
            value={overrides[field] ?? ""}
            onChange={(event) =>
              onSet(
                field,
                event.target.value === "" ? null : event.target.value,
              )
            }
          />
        </Field>
      ))}
      {LIST_FIELDS.map((field) => (
        <Field key={field} label={`${field} (comma separated)`}>
          <CommitInput
            label={field}
            value={formatList(overrides[field])}
            onCommit={(text) => onSet(field, parseList(text))}
          />
        </Field>
      ))}
      {FLAG_FIELDS.map((field) => (
        <Field key={field} label={field}>
          <TriStateSelect
            label={field}
            value={overrides[field]}
            onChange={(value) => onSet(field, value)}
          />
        </Field>
      ))}
    </div>
  );
}
