import { ChevronLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { BACK_LABEL, breadcrumbLabel } from "@/lib/labels";
import { useNavStore } from "@/stores/nav";

// A quiet strip above the page content — only worth showing at all once a
// cross-page link has actually left a trail to come back to. Landing on a
// page straight from the sidebar has nowhere to go back to, so nothing here
// reserves space for it.
export function NavBar() {
  const page = useNavStore((s) => s.page);
  const libraryTab = useNavStore((s) => s.libraryTab);
  const toolsTab = useNavStore((s) => s.toolsTab);
  const hasHistory = useNavStore((s) => s.history.length > 0);
  const back = useNavStore((s) => s.back);

  if (!hasHistory) return null;

  return (
    <div className="flex items-center gap-1 px-8 pt-3 text-xs text-muted-foreground">
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label={BACK_LABEL}
        title={BACK_LABEL}
        onClick={back}
      >
        <ChevronLeft className="size-3.5" />
      </Button>
      <span>{breadcrumbLabel({ page, libraryTab, toolsTab })}</span>
    </div>
  );
}
