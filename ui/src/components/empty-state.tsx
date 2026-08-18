import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

/**
 * A page with nothing in it, said once and said well: a mark, the good news
 * in a sentence, and — where there is one — the single thing worth doing
 * from here. Centred in the space it has, because a line of grey text hung
 * off the left margin reads as something missing rather than as an answer.
 */
export function EmptyState({
  icon: Icon,
  title,
  children,
  action,
}: {
  icon: LucideIcon;
  title: string;
  /** One line. Anything longer belongs on the page, not in its empty state. */
  children?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center gap-3 px-6 py-20 text-center">
      <span className="grid size-14 place-items-center rounded-full bg-muted/60 text-muted-foreground">
        <Icon className="size-6" />
      </span>
      <p className="text-base font-semibold tracking-tight">{title}</p>
      {children ? (
        <p className="max-w-sm text-[13px] text-muted-foreground">{children}</p>
      ) : null}
      {action ? <div className="mt-1">{action}</div> : null}
    </div>
  );
}
