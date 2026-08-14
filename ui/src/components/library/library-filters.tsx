import type { RefObject } from "react";
import type { HarnessId, ItemKind } from "@/bindings";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { kindLabel, toolName } from "@/lib/labels";

const KINDS: ItemKind[] = [
  "agent",
  "skill",
  "hook",
  "command",
  "mcp-server",
  "plugin",
  "pi-extension",
];
const HARNESSES: HarnessId[] = [
  "claude",
  "codex",
  "opencode",
  "cursor",
  "pi",
  "gemini",
  "copilot",
];

export function LibraryFilters({
  searchRef,
  search,
  onSearchChange,
  kind,
  onKindChange,
  harness,
  onHarnessChange,
}: {
  searchRef: RefObject<HTMLInputElement | null>;
  search: string;
  onSearchChange: (value: string) => void;
  kind: string;
  onKindChange: (value: string) => void;
  harness: string;
  onHarnessChange: (value: string) => void;
}) {
  return (
    <div className="border-b px-8 py-3">
      <div className="mx-auto flex w-full max-w-5xl gap-2">
        <div className="relative max-w-56 flex-1">
          <Input
            ref={searchRef}
            placeholder="Search by name…"
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
            className="pr-8"
          />
          <kbd className="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
            /
          </kbd>
        </div>
        <Select
          value={kind}
          onValueChange={(value) => onKindChange(value ?? kind)}
        >
          <SelectTrigger className="w-40">
            <SelectValue>
              {(value: string) =>
                value === "any" ? "All types" : kindLabel(value as ItemKind, 2)
              }
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="any">All types</SelectItem>
            {KINDS.map((k) => (
              <SelectItem key={k} value={k}>
                {kindLabel(k, 2)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select
          value={harness}
          onValueChange={(value) => onHarnessChange(value ?? harness)}
        >
          <SelectTrigger className="w-40">
            <SelectValue>
              {(value: string) =>
                value === "any" ? "All tools" : toolName(value as HarnessId)
              }
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="any">All tools</SelectItem>
            {HARNESSES.map((h) => (
              <SelectItem key={h} value={h}>
                {toolName(h)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
