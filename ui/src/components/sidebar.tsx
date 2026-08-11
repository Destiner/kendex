import {
  FolderTree,
  GitBranch,
  Home,
  Library,
  RefreshCw,
  Settings,
  ShieldAlert,
  SlidersHorizontal,
  TerminalSquare,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { projectScopes } from "@/lib/derive";
import { scopeName } from "@/lib/labels";
import { cn } from "@/lib/utils";
import { useAuditStore } from "@/stores/audit";
import { type Page, useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";

const NAV: { page: Page; label: string; icon: typeof Home }[] = [
  { page: "overview", label: "Home", icon: Home },
  { page: "items", label: "Library", icon: Library },
  { page: "harnesses", label: "Tools", icon: TerminalSquare },
  { page: "scopes", label: "Projects", icon: FolderTree },
  { page: "audit", label: "Sync", icon: ShieldAlert },
  { page: "sources", label: "Catalogs", icon: GitBranch },
  { page: "customize", label: "Customize", icon: SlidersHorizontal },
  { page: "settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const { page, scope, setPage, setScope } = useNavStore();
  const { result, scanning, refresh } = useScanStore();
  const driftCount = useAuditStore((s) =>
    s.views.reduce((sum, view) => sum + view.drift.length, 0),
  );
  const projects = result ? projectScopes(result) : [];

  const scopeValue =
    scope === "all" ? "all" : scope === "global" ? "global" : scope.project;

  return (
    <aside className="flex h-full w-56 shrink-0 flex-col border-r bg-sidebar text-sidebar-foreground">
      <div className="flex items-center justify-between px-4 py-4">
        <span className="font-semibold tracking-tight">vstack</span>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Scan again"
          title="Scan again"
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
            <span className="flex-1 text-left">{label}</span>
            {target === "audit" && driftCount > 0 ? (
              <Badge variant="secondary">{driftCount}</Badge>
            ) : null}
          </button>
        ))}
      </nav>
      <div className="border-t px-3 py-3">
        <p className="mb-1 px-1 text-xs text-muted-foreground">Show</p>
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
            <SelectItem value="all">Everything</SelectItem>
            <SelectItem value="global">Global — this machine</SelectItem>
            {projects.map((root) => (
              <SelectItem key={root} value={root}>
                {scopeName({ scope: "project", root })}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </aside>
  );
}
