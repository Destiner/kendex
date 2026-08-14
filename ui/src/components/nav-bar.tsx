import { ChevronLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { BACK_LABEL, breadcrumbLabel } from "@/lib/labels";
import { useNavStore } from "@/stores/nav";

// A quiet strip above the page content — only worth showing the back
// button when there's actually somewhere to go back to.
export function NavBar() {
  const page = useNavStore((s) => s.page);
  const libraryTab = useNavStore((s) => s.libraryTab);
  const toolsTab = useNavStore((s) => s.toolsTab);
  const hasHistory = useNavStore((s) => s.history.length > 0);
  const back = useNavStore((s) => s.back);

  return (
    <div className="flex items-center gap-1 px-8 pt-3 text-xs text-muted-foreground">
      {hasHistory ? (
        <Button
          variant="ghost"
          size="icon-xs"
          aria-label={BACK_LABEL}
          title={BACK_LABEL}
          onClick={back}
        >
          <ChevronLeft className="size-3.5" />
        </Button>
      ) : null}
      <span>{breadcrumbLabel({ page, libraryTab, toolsTab })}</span>
    </div>
  );
}
