import type { HarnessId, ItemKind, Tag } from "@/bindings";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { Location } from "@/lib/derive";
import { kindLabel, scopeName, TAG_LABELS, toolName } from "@/lib/labels";
import { PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
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
// Derived, not written out again: a tag missing from the filter is a tag
// nobody can find, and nothing would have caught a hand-kept list drifting.
const TAGS = Object.keys(TAG_LABELS) as Tag[];
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
  kind,
  onKindChange,
  harness,
  onHarnessChange,
  tag,
  onTagChange,
  locations,
  onLocationsChange,
  projects,
}: {
  kind: string;
  onKindChange: (value: string) => void;
  harness: string;
  onHarnessChange: (value: string) => void;
  tag: string;
  onTagChange: (value: string) => void;
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
    <div className={cn("border-b py-3", PAGE_GUTTER)}>
      {/* Narrower windows (900px, the density check point) can't fit search
          plus three pickers at full width — this scrolls sideways instead
          of squeezing the search box down to unreadable, à la the table's
          own overflow wrapper. */}
      <div className={cn("flex flex-col gap-3", WIDE_CONTENT_WIDTH)}>
        <div className="flex gap-2 overflow-x-auto">
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
            value={tag}
            onValueChange={(value) => onTagChange(value ?? tag)}
          >
            <SelectTrigger className="w-40 shrink-0">
              <SelectValue>
                {(value: string) =>
                  value === "any" ? "All tags" : TAG_LABELS[value as Tag]
                }
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="any">All tags</SelectItem>
              {TAGS.map((t) => (
                <SelectItem key={t} value={t}>
                  {TAG_LABELS[t]}
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
        // Selection is the loudest thing about the pill: a fill only a
        // shade lighter than the page, against an outlined neighbour, reads
        // backwards.
        selected
          ? "border-transparent bg-primary/20 text-primary"
          : "border-border text-muted-foreground hover:border-input hover:text-foreground",
      )}
    >
      {label}
    </button>
  );
}
