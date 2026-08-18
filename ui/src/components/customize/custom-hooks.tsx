import { Plus, X } from "lucide-react";
import { CommitInput, Field } from "@/components/customize/controls";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  addCustomHook,
  type Draft,
  type DraftHook,
  formatHookAgents,
  parseHookAgents,
  removeCustomHook,
  setCustomHook,
} from "@/lib/editor-draft";

export function CustomHooks({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (change: (draft: Draft) => Draft) => void;
}) {
  const hooks = draft["custom-hooks"] ?? [];

  return (
    <div className="space-y-3">
      {hooks.map((hook, index) => (
        <HookCard
          // biome-ignore lint/suspicious/noArrayIndexKey: position is a hook's only identity — [[custom-hooks]] is an ordered list with no names
          key={index}
          hook={hook}
          onEdit={(next) =>
            onChange((current) => setCustomHook(current, index, next))
          }
          onRemove={() =>
            onChange((current) => removeCustomHook(current, index))
          }
        />
      ))}
      <Button
        variant="outline"
        size="sm"
        onClick={() => onChange((current) => addCustomHook(current))}
      >
        <Plus className="size-4" />
        Add hook
      </Button>
    </div>
  );
}

function HookCard({
  hook,
  onEdit,
  onRemove,
}: {
  hook: DraftHook;
  onEdit: (hook: DraftHook) => void;
  onRemove: () => void;
}) {
  const optional = (text: string) => (text === "" ? null : text);

  return (
    <Card>
      <CardContent className="grid gap-3 sm:grid-cols-2">
        <Field label="Event">
          <Input
            aria-label="Event"
            placeholder="PreToolUse"
            value={hook.event}
            onChange={(e) => onEdit({ ...hook, event: e.target.value })}
          />
        </Field>
        <Field label="Matcher (optional)">
          <Input
            aria-label="Matcher"
            placeholder="Bash"
            value={hook.matcher ?? ""}
            onChange={(e) =>
              onEdit({ ...hook, matcher: optional(e.target.value) })
            }
          />
        </Field>
        <Field label="Command">
          <Input
            aria-label="Command"
            placeholder="./guard.sh"
            value={hook.command}
            onChange={(e) => onEdit({ ...hook, command: e.target.value })}
          />
        </Field>
        <Field label="Description (optional)">
          <Input
            aria-label="Description"
            value={hook.description ?? ""}
            onChange={(e) =>
              onEdit({ ...hook, description: optional(e.target.value) })
            }
          />
        </Field>
        <Field label="Agents — all, a role, or a comma-separated list">
          <CommitInput
            label="Agents"
            placeholder="all"
            value={formatHookAgents(hook.agents)}
            onCommit={(text) =>
              onEdit({ ...hook, agents: parseHookAgents(text) })
            }
          />
        </Field>
        <div className="flex items-end justify-end">
          <Button
            variant="ghost"
            size="sm"
            aria-label="Remove hook"
            onClick={onRemove}
          >
            <X className="size-4" />
            Remove
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
