import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/** A clickable number-over-label tile, for at-a-glance stat strips. */
export function StatTile({
  label,
  value,
  detail,
  onClick,
}: {
  label: string;
  value: number;
  detail?: ReactNode;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={!onClick}
      className={cn(
        "rounded-lg border px-4 py-3 text-left transition-colors",
        onClick && "cursor-pointer hover:bg-accent",
      )}
    >
      <p className="text-2xl font-semibold tracking-tight">{value}</p>
      <p className="text-xs font-medium tracking-widest text-muted-foreground uppercase">
        {label}
      </p>
      {detail ? (
        <div className="mt-1 truncate text-xs text-muted-foreground">
          {detail}
        </div>
      ) : null}
    </button>
  );
}
