import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * One choice in a row of them — a scope, a tool. Distinct from a tab on
 * purpose: tabs switch a page's section, pills narrow what a section is
 * showing, and a page that uses tab bars for both reads as broken.
 */
export function Pill({
  selected,
  title,
  disabled,
  onClick,
  children,
}: {
  selected: boolean;
  title?: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      title={title}
      disabled={disabled && !selected}
      onClick={onClick}
      className={cn(
        "inline-flex h-7 shrink-0 items-center gap-1.5 rounded-full border px-3 text-xs font-medium transition-colors",
        // Selection is the loudest thing about the pill: a fill only a
        // shade lighter than the page, against an outlined neighbour, reads
        // backwards.
        selected
          ? "border-transparent bg-primary/20 text-selected"
          : "border-border text-muted-foreground hover:border-input hover:text-foreground",
        "disabled:pointer-events-none disabled:opacity-50",
      )}
    >
      {children}
    </button>
  );
}
