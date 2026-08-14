import { useEffect } from "react";
import { NavBar } from "@/components/nav-bar";
import { Sidebar } from "@/components/sidebar";
import { Titlebar } from "@/components/titlebar";
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
    void load().then(refresh).then(auditRefresh);
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

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <Titlebar />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <main className="flex flex-1 flex-col overflow-hidden">
          <NavBar />
          <div className="flex-1 overflow-y-auto">
            {page === "home" && <OverviewPage />}
            {page === "library" && <LibraryPage />}
            {page === "tools" && <ToolsProjectsPage />}
            {page === "review" && <ReviewPage />}
            {page === "customize" && <CustomizePage />}
            {page === "settings" && <SettingsPage />}
          </div>
        </main>
      </div>
    </div>
  );
}
