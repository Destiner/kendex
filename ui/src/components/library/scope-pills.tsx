import type { ScopeSelection } from "@/lib/derive";
import { scopeName } from "@/lib/labels";
import { cn } from "@/lib/utils";

/** Where the table is looking: the same app-wide scope the sidebar sets, so
 *  the two can never disagree about which project is on screen. */
export function ScopePills({
  scope,
  onScopeChange,
  projects,
}: {
  scope: ScopeSelection;
  onScopeChange: (scope: ScopeSelection) => void;
  /** Project roots that currently have at least one item. */
  projects: string[];
}) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
      <span className="text-xs text-muted-foreground">Where</span>
      <div className="flex flex-wrap gap-1.5">
        <ScopePill
          label="Everywhere"
          selected={scope === "all"}
          onClick={() => onScopeChange("all")}
        />
        <ScopePill
          label="Personal"
          selected={scope === "global"}
          onClick={() => onScopeChange("global")}
        />
        {projects.map((root) => (
          <ScopePill
            key={root}
            label={scopeName({ scope: "project", root })}
            title={root}
            selected={
              scope !== "all" && scope !== "global" && scope.project === root
            }
            onClick={() => onScopeChange({ project: root })}
          />
        ))}
      </div>
    </div>
  );
}

function ScopePill({
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
        "inline-flex h-7 shrink-0 items-center rounded-full border px-3 text-xs font-medium transition-colors",
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
