import {
  Boxes,
  FolderTree,
  Home,
  RefreshCw,
  Settings,
  TerminalSquare,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { projectScopes } from "@/lib/derive";
import { cn } from "@/lib/utils";
import { type Page, useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";

const NAV: { page: Page; label: string; icon: typeof Home }[] = [
  { page: "overview", label: "Overview", icon: Home },
  { page: "items", label: "Items", icon: Boxes },
  { page: "harnesses", label: "Harnesses", icon: TerminalSquare },
  { page: "scopes", label: "Scopes", icon: FolderTree },
  { page: "settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const { page, scope, setPage, setScope } = useNavStore();
  const { result, scanning, refresh } = useScanStore();
  const projects = result ? projectScopes(result) : [];

  const scopeValue =
    scope === "all" ? "all" : scope === "global" ? "global" : scope.project;

  return (
    <aside className="flex h-full w-56 shrink-0 flex-col border-r bg-sidebar text-sidebar-foreground">
      <div className="flex items-center justify-between px-4 py-4">
        <span className="font-semibold tracking-tight">vstack2</span>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Rescan"
          onClick={() => void refresh()}
          disabled={scanning}
        >
          <RefreshCw className={cn("size-4", scanning && "animate-spin")} />
        </Button>
      </div>
      <nav className="flex flex-1 flex-col gap-1 px-2">
        {NAV.map(({ page: target, label, icon: Icon }) => (
          <button
            key={target}
            type="button"
            onClick={() => setPage(target)}
            className={cn(
              "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
              page === target
                ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground"
                : "text-muted-foreground hover:bg-sidebar-accent/60",
            )}
          >
            <Icon className="size-4" />
            {label}
          </button>
        ))}
      </nav>
      <div className="border-t px-3 py-3">
        <p className="mb-1 px-1 text-xs text-muted-foreground">Scope</p>
        <Select
          value={scopeValue}
          onValueChange={(value) =>
            setScope(
              value === "all"
                ? "all"
                : value === "global"
                  ? "global"
                  : { project: value },
            )
          }
        >
          <SelectTrigger className="w-full" size="sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All scopes</SelectItem>
            <SelectItem value="global">Global</SelectItem>
            {projects.map((root) => (
              <SelectItem key={root} value={root}>
                {root.split("/").pop() ?? root}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </aside>
  );
}
