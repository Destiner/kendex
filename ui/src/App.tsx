import { useEffect } from "react";
import { Toaster } from "sonner";
import { commands } from "@/bindings";
import { NavBar } from "@/components/nav-bar";
import { Sidebar } from "@/components/sidebar";
import { StatusFooter } from "@/components/status-footer";
import { WindowControls } from "@/components/window-controls";
import { CustomizePage } from "@/pages/customize";
import { LibraryPage } from "@/pages/library";
import { OverviewPage } from "@/pages/overview";
import { ReviewPage } from "@/pages/review";
import { SettingsPage } from "@/pages/settings";
import { ToolsProjectsPage } from "@/pages/tools";
import { useAuditStore } from "@/stores/audit";
import { useNavStore } from "@/stores/nav";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

const FOCUS_RESCAN_DEBOUNCE_MS = 5000;

function useAppearance() {
  const appearance = useSettingsStore(
    (s) => s.settings?.appearance ?? "system",
  );
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark =
        appearance === "dark" || (appearance === "system" && media.matches);
      document.documentElement.classList.toggle("dark", dark);
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [appearance]);
}

function useScanTriggers() {
  const refresh = useScanStore((s) => s.refresh);
  const auditRefresh = useAuditStore((s) => s.refresh);
  const load = useSettingsStore((s) => s.load);
  useEffect(() => {
    void load()
      .then(() => refresh())
      .then(auditRefresh);
    let last = Date.now();
    const onFocus = () => {
      if (Date.now() - last < FOCUS_RESCAN_DEBOUNCE_MS) return;
      last = Date.now();
      void refresh();
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refresh, auditRefresh, load]);
}

export default function App() {
  useAppearance();
  useScanTriggers();
  const page = useNavStore((s) => s.page);
  const appearance = useSettingsStore(
    (s) => s.settings?.appearance ?? "system",
  );

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <Toaster
        theme={appearance}
        position="bottom-right"
        offset={{ bottom: "2rem" }}
        toastOptions={{
          classNames: {
            toast:
              "!bg-popover !text-popover-foreground !border-border !shadow-lg",
            title: "!text-sm !font-medium",
            description: "!text-muted-foreground",
            actionButton: "!bg-primary !text-primary-foreground",
            cancelButton: "!bg-muted !text-muted-foreground",
          },
        }}
      />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <main className="relative flex flex-1 flex-col overflow-hidden">
          {/* biome-ignore lint/a11y/noStaticElementInteractions: double-click here is a convenience alias for the maximize button already on screen */}
          <div
            data-tauri-drag-region
            onDoubleClick={() => void commands.windowToggleMaximize()}
            className="absolute inset-x-0 top-0 h-8"
          />
          <WindowControls className="absolute top-0 right-0 z-20" />
          {/* Above the drag strip so nothing real content renders ever sits
              under it, no matter which page or nav state is showing. */}
          <div className="relative z-10 flex flex-1 flex-col overflow-hidden">
            <NavBar />
            <div className="flex-1 overflow-y-auto">
              {page === "home" && <OverviewPage />}
              {page === "library" && <LibraryPage />}
              {page === "tools" && <ToolsProjectsPage />}
              {page === "review" && <ReviewPage />}
              {page === "customize" && <CustomizePage />}
              {page === "settings" && <SettingsPage />}
            </div>
          </div>
        </main>
      </div>
      <StatusFooter />
    </div>
  );
}
