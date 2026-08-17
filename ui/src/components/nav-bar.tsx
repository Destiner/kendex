import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { BACK_LABEL, FORWARD_LABEL } from "@/lib/copy";
import { breadcrumbLabel, packageDisplayName } from "@/lib/labels";
import {
  CONTENT_WIDTH,
  isWidePage,
  PAGE_GUTTER,
  WIDE_CONTENT_WIDTH,
} from "@/lib/layout";
import { cn } from "@/lib/utils";
import { useNavStore } from "@/stores/nav";

// A quiet strip above the page content — only worth showing at all once a
// cross-page link has actually left a trail to come back to. Landing on a
// page straight from the sidebar has nowhere to go back to, so nothing here
// reserves space for it.
export function NavBar() {
  const page = useNavStore((s) => s.page);
  const libraryTab = useNavStore((s) => s.libraryTab);
  const toolsTab = useNavStore((s) => s.toolsTab);
  const packageRef = useNavStore((s) => s.packageRef);
  const hasHistory = useNavStore((s) => s.history.length > 0);
  const hasFuture = useNavStore((s) => s.future.length > 0);
  const back = useNavStore((s) => s.back);
  const forward = useNavStore((s) => s.forward);

  if (!hasHistory && !hasFuture) return null;

  return (
    // Same gutters and measure as the page below, so the back button lines
    // up with the title it belongs to instead of floating off to its left.
    <div className={cn("pt-3", PAGE_GUTTER)}>
      <div
        className={cn(
          "flex items-center gap-0.5 text-xs text-muted-foreground",
          isWidePage(page) ? WIDE_CONTENT_WIDTH : CONTENT_WIDTH,
        )}
      >
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={BACK_LABEL}
          title={BACK_LABEL}
          disabled={!hasHistory}
          onClick={back}
        >
          <ChevronLeft className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={FORWARD_LABEL}
          title={FORWARD_LABEL}
          disabled={!hasFuture}
          onClick={forward}
        >
          <ChevronRight className="size-4" />
        </Button>
        <span className="ml-1.5 min-w-0 truncate">
          {breadcrumbLabel({
            page,
            libraryTab,
            toolsTab,
            packageName: packageRef ? packageDisplayName(packageRef) : null,
          })}
        </span>
      </div>
    </div>
  );
}
