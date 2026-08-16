import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { FileDiff } from "@/bindings";
import { additionsLabel, DIFF_STATUS_LABELS, deletionsLabel } from "@/lib/diff";
import { cn } from "@/lib/utils";

/** One changed file: a disclosure row with its counts, opening to the
 *  unified diff. Lines ride the semantic good/critical tokens — never raw
 *  green/red — so both themes read the same way the rest of the app does. */
export function DiffFile({
  file,
  gutterCh,
  defaultOpen,
}: {
  file: FileDiff;
  gutterCh: number;
  defaultOpen: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen && file.hunks.length > 0);
  const Chevron = open ? ChevronDown : ChevronRight;
  const status = DIFF_STATUS_LABELS[file.status];
  const expandable = file.hunks.length > 0;

  return (
    <div className="overflow-hidden rounded-lg border">
      <button
        type="button"
        className={cn(
          "flex w-full items-center gap-2 px-3 py-2 text-left",
          expandable && "hover:bg-accent/50",
        )}
        onClick={() => expandable && setOpen((value) => !value)}
        disabled={!expandable}
      >
        {expandable ? (
          <Chevron className="size-3.5 shrink-0 text-muted-foreground" />
        ) : (
          <span className="w-3.5 shrink-0" />
        )}
        <span className="min-w-0 truncate font-mono text-xs">{file.path}</span>
        {status ? (
          <span className="shrink-0 text-xs text-muted-foreground">
            {status}
          </span>
        ) : null}
        <span className="ml-auto flex shrink-0 gap-2 font-mono text-xs tabular-nums">
          {file.additions > 0 ? (
            <span className="text-good">{additionsLabel(file.additions)}</span>
          ) : null}
          {file.deletions > 0 ? (
            <span className="text-critical">
              {deletionsLabel(file.deletions)}
            </span>
          ) : null}
        </span>
      </button>
      {open ? (
        <div className="overflow-x-auto border-t">
          {file.hunks.map((hunk) => (
            <div key={hunk.header}>
              <div className="bg-muted/40 px-3 py-1 font-mono text-[11px] text-muted-foreground">
                {hunk.header}
              </div>
              {hunk.lines.map((line) => (
                <div
                  // Within a hunk every line carries at least one line
                  // number, and no two lines share the same pair.
                  key={`${line.oldNo ?? "a"}:${line.newNo ?? "r"}`}
                  className={cn(
                    "flex font-mono text-xs leading-5",
                    line.kind === "add" && "bg-good/10",
                    line.kind === "remove" && "bg-critical/10",
                  )}
                >
                  <span
                    className="shrink-0 select-none pr-2 text-right text-muted-foreground/60"
                    style={{ width: `${gutterCh + 1.5}ch` }}
                  >
                    {line.oldNo ?? ""}
                  </span>
                  <span
                    className="shrink-0 select-none pr-2 text-right text-muted-foreground/60"
                    style={{ width: `${gutterCh + 1.5}ch` }}
                  >
                    {line.newNo ?? ""}
                  </span>
                  <span
                    className={cn(
                      "w-4 shrink-0 select-none text-center",
                      line.kind === "add" && "text-good",
                      line.kind === "remove" && "text-critical",
                    )}
                  >
                    {line.kind === "add"
                      ? "+"
                      : line.kind === "remove"
                        ? "−"
                        : ""}
                  </span>
                  <span className="whitespace-pre pr-3">{line.text}</span>
                </div>
              ))}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
