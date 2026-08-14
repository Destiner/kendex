import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

/**
 * The category label above a card's content group — "Appearance",
 * "Projects", "Add a project" — set apart from the row text beneath it so a
 * card reads as one hierarchy (label → primary text → description) instead
 * of two same-weight headings stacked on top of each other.
 *
 * Pairs with a tightened `Card`: `<Card className="gap-3 py-4">` and
 * `<CardHeader className="gap-1">` so the label sits close to the content
 * it names, Vercel-dashboard style, rather than floating a `gap-4` away
 * from it.
 */
export function SectionLabel({ className, ...props }: ComponentProps<"h3">) {
  return (
    <h3
      className={cn(
        "text-[11px] font-medium uppercase tracking-wider text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}
