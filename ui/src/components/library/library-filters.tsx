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
import type { Location } from "@/lib/derive";
import { kindLabel, scopeName, toolName } from "@/lib/labels";
import { cn } from "@/lib/utils";

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
  locations,
  onLocationsChange,
  projects,
}: {
  searchRef: RefObject<HTMLInputElement | null>;
  search: string;
  onSearchChange: (value: string) => void;
  kind: string;
  onKindChange: (value: string) => void;
  harness: string;
  onHarnessChange: (value: string) => void;
  /** Empty set is "All" — the pills are a multi-select on top of it. */
  locations: ReadonlySet<Location>;
  onLocationsChange: (locations: Set<Location>) => void;
  /** Project roots that currently have at least one item, for the pills. */
  projects: string[];
}) {
  const toggleLocation = (location: Location) => {
    const next = new Set(locations);
    if (next.has(location)) next.delete(location);
    else next.add(location);
    onLocationsChange(next);
  };

  return (
    <div className="border-b px-8 py-3">
      {/* Narrower windows (900px, the density check point) can't fit search
          plus three pickers at full width — this scrolls sideways instead
          of squeezing the search box down to unreadable, à la the table's
          own overflow wrapper. */}
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-2">
        <div className="flex gap-2 overflow-x-auto">
          <div className="relative w-56 shrink-0">
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
            <SelectTrigger className="w-40 shrink-0">
              <SelectValue>
                {(value: string) =>
                  value === "any"
                    ? "All types"
                    : kindLabel(value as ItemKind, 2)
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
            <SelectTrigger className="w-40 shrink-0">
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
        {projects.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            <LocationPill
              label="All"
              selected={locations.size === 0}
              onClick={() => onLocationsChange(new Set())}
            />
            <LocationPill
              label="Personal"
              selected={locations.has("global")}
              onClick={() => toggleLocation("global")}
            />
            {projects.map((root) => (
              <LocationPill
                key={root}
                label={scopeName({ scope: "project", root })}
                title={root}
                selected={locations.has(root)}
                onClick={() => toggleLocation(root)}
              />
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}

function LocationPill({
  label,
  title,
  selected,
  onClick,
}: {
  label: string;
  title?: string;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      title={title}
      onClick={onClick}
      className={cn(
        "inline-flex h-6 shrink-0 items-center rounded-full border px-2.5 text-xs font-medium transition-colors",
        selected
          ? "border-transparent bg-secondary text-foreground"
          : "border-input text-muted-foreground hover:text-foreground",
      )}
    >
      {label}
    </button>
  );
}
