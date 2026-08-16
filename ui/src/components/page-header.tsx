import type { ReactNode } from "react";
import { CONTENT_WIDTH, PAGE_GUTTER, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { cn } from "@/lib/utils";

export function PageHeader({
  title,
  subtitle,
  action,
  wide = false,
}: {
  title: string;
  subtitle?: string;
  action?: ReactNode;
  /** Line the header up with a page that runs full-width, not to the reading cap. */
  wide?: boolean;
}) {
  return (
    <header className={cn("pt-8 pb-6", PAGE_GUTTER)}>
      <div
        className={cn(
          "flex items-center justify-between gap-4",
          wide ? WIDE_CONTENT_WIDTH : CONTENT_WIDTH,
        )}
      >
        <div className="min-w-0">
          <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
          {subtitle ? (
            <p className="mt-1 text-sm text-muted-foreground">{subtitle}</p>
          ) : null}
        </div>
        {action ? <div className="flex shrink-0 gap-2">{action}</div> : null}
      </div>
    </header>
  );
}
