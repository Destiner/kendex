import { useEffect } from "react";
import { Sidebar } from "@/components/sidebar";
import { AuditPage } from "@/pages/audit";
import { CustomizePage } from "@/pages/customize";
import { HarnessesPage } from "@/pages/harnesses";
import { ItemsPage } from "@/pages/items";
import { OverviewPage } from "@/pages/overview";
import { ScopesPage } from "@/pages/scopes";
import { SettingsPage } from "@/pages/settings";
import { SourcesPage } from "@/pages/sources";
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
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        {page === "overview" && <OverviewPage />}
        {page === "items" && <ItemsPage />}
        {page === "harnesses" && <HarnessesPage />}
        {page === "scopes" && <ScopesPage />}
        {page === "audit" && <AuditPage />}
        {page === "sources" && <SourcesPage />}
        {page === "customize" && <CustomizePage />}
        {page === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}
