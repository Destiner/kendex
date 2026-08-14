import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { heldBackCount } from "@/lib/derive";
import {
  heldBackFooterLabel,
  pendingChangesLabel,
  SCANNING_LABEL,
  scanStatusLabel,
} from "@/lib/labels";
import { relativeTime } from "@/lib/relative-time";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";

const AGE_TICK_MS = 30_000;

// A persistent strip across the whole window, not just the content pane —
// scan freshness and pending-changes counts apply regardless of which page
// you're looking at.
export function StatusFooter() {
  const scanning = useScanStore((s) => s.scanning);
  const lastScanAt = useScanStore((s) => s.lastScanAt);
  const views = useAuditStore((s) => s.views);
  const goTo = useNavStore((s) => s.goTo);

  // "Scanned Nm ago" goes stale on its own; nothing else re-renders this
  // component often enough to keep it honest.
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), AGE_TICK_MS);
    return () => clearInterval(id);
  }, []);

  const pending = views.reduce((sum, view) => sum + view.drift.length, 0);
  const held = heldBackCount(views);

  return (
    <footer className="flex h-7 shrink-0 items-center justify-between border-t bg-background px-4 text-xs text-muted-foreground">
      <span className="flex items-center gap-1.5">
        {scanning ? (
          <>
            <RefreshCw className="size-3 animate-spin" />
            {SCANNING_LABEL}
          </>
        ) : (
          scanStatusLabel(lastScanAt ? relativeTime(lastScanAt, now) : null)
        )}
      </span>
      <span className="flex items-center gap-3">
        {pending > 0 ? (
          <button
            type="button"
            className="hover:text-foreground"
            onClick={() => goTo("review")}
          >
            {pendingChangesLabel(pending)}
          </button>
        ) : null}
        {held > 0 ? (
          <button
            type="button"
            className="hover:text-foreground"
            onClick={() => goTo("review")}
          >
            {heldBackFooterLabel(held)}
          </button>
        ) : null}
      </span>
    </footer>
  );
}
