import { Plus, X } from "lucide-react";
import type { EditorInventory } from "@/bindings";
import { CommitInput, Field } from "@/components/customize/controls";
import { EventPicker } from "@/components/customize/event-picker";
import { HarnessIcon } from "@/components/harness-icon";
import { StatusLine } from "@/components/status-note";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  ADVISORY_EVERYWHERE_ELSE,
  HOOK_AGENTS_LABEL,
  HOOK_COMMAND_HELP,
  HOOK_COMMAND_PLACEHOLDER,
  hookRunBy,
  MATCHER_HELP,
} from "@/lib/copy-customize";
import {
  addCustomHook,
  type Draft,
  type DraftHook,
  formatHookAgents,
  parseHookAgents,
  removeCustomHook,
  setCustomHook,
} from "@/lib/editor-draft";
import { harnessName } from "@/lib/labels";

export function CustomHooks({
  draft,
  inventory,
  onChange,
}: {
  draft: Draft;
  inventory: EditorInventory | null;
  onChange: (change: (draft: Draft) => Draft) => void;
}) {
  const hooks = draft["custom-hooks"] ?? [];
  const enforcedBy = inventory?.hookEnforcedBy ?? [];

  return (
    <div className="flex flex-col gap-3">
      {hooks.length > 0 && enforcedBy.length > 0 ? (
        // A guard that only asks nicely, presented as a guard, is worse
        // than no guard — so where a hook runs is said once, up front.
        <StatusLine tone="info">
          <span className="inline-flex items-center gap-1.5">
            {enforcedBy.map((harness) => (
              <HarnessIcon
                key={harness}
                harness={harness}
                className="size-3.5"
              />
            ))}
            {hookRunBy(enforcedBy.map(harnessName))} {ADVISORY_EVERYWHERE_ELSE}
          </span>
        </StatusLine>
      ) : null}
      {hooks.map((hook, index) => (
        <HookCard
          // biome-ignore lint/suspicious/noArrayIndexKey: position is a hook's only identity — [[custom-hooks]] is an ordered list with no names
          key={index}
          hook={hook}
          inventory={inventory}
          onEdit={(next) =>
            onChange((current) => setCustomHook(current, index, next))
          }
          onRemove={() =>
            onChange((current) => removeCustomHook(current, index))
          }
        />
      ))}
      <div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => onChange((current) => addCustomHook(current))}
        >
          <Plus className="size-4" />
          Add hook
        </Button>
      </div>
    </div>
  );
}

function HookCard({
  hook,
  inventory,
  onEdit,
  onRemove,
}: {
  hook: DraftHook;
  inventory: EditorInventory | null;
  onEdit: (hook: DraftHook) => void;
  onRemove: () => void;
}) {
  const optional = (text: string) => (text === "" ? null : text);

  return (
    <Card>
      <CardContent className="grid gap-3 sm:grid-cols-2">
        <Field label="Event">
          <EventPicker
            value={hook.event}
            events={inventory?.hookEvents ?? []}
            onPick={(event) => onEdit({ ...hook, event })}
          />
        </Field>
        <Field label={MATCHER_HELP}>
          <Input
            aria-label="Matcher"
            placeholder="Bash"
            value={hook.matcher ?? ""}
            onChange={(e) =>
              onEdit({ ...hook, matcher: optional(e.target.value) })
            }
          />
        </Field>
        <Field label={HOOK_COMMAND_HELP}>
          <Input
            aria-label="Command"
            placeholder={HOOK_COMMAND_PLACEHOLDER}
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
        <Field label={HOOK_AGENTS_LABEL}>
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
