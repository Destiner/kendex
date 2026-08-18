import type { ReactNode } from "react";
import { CONTENT_WIDTH, PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { useNavStore } from "@/stores/nav";

export function PageHeader({
  title,
  subtitle,
  action,
  wide = false,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  action?: ReactNode;
  /** Line the header up with a page that runs full-width, not to the reading cap. */
  wide?: boolean;
}) {
  // A page starts well below the window's edge, so the title is never the
  // first thing the frame touches. The back strip, when there is one, takes
  // that room instead — otherwise the two would stack and push the title
  // into the middle of the page.
  const backStrip = useNavStore((s) => s.history.length > 0);
  return (
    <header className={cn(backStrip ? "pt-8" : "pt-20", "pb-6", PAGE_GUTTER)}>
      <div className={cn(wide ? WIDE_CONTENT_WIDTH : CONTENT_WIDTH)}>
        {/* Actions belong beside the title, not beside the description: a
            description can run to eight lines, and buttons centred against
            that end up floating in the middle of the page. */}
        <div className="flex items-start justify-between gap-4">
          <h1 className="min-w-0 text-2xl font-semibold tracking-tight">
            {title}
          </h1>
          {action ? (
            <div className="flex shrink-0 items-center gap-2">{action}</div>
          ) : null}
        </div>
        {subtitle ? (
          // Capped at a reading measure even on a full-width page — prose
          // that runs the whole window is prose nobody finishes a line of.
          <div className="mt-2 max-w-3xl text-sm leading-relaxed text-muted-foreground">
            {subtitle}
          </div>
        ) : null}
      </div>
    </header>
  );
}
