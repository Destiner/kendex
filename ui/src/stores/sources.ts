import { create } from "zustand";
import { commands, type Scope, type SourceRow } from "@/bindings";
import { useAuditStore } from "./audit";
import { useScanStore } from "./scan";

interface SourcesState {
  rows: SourceRow[];
  busy: boolean;
  error: string | null;
  warnings: string[];
  load: () => Promise<void>;
  add: (scope: Scope, name: string, reference: string) => Promise<void>;
  remove: (scope: Scope, name: string) => Promise<void>;
  toggle: (scope: Scope, name: string, enabled: boolean) => Promise<void>;
  refreshRemotes: () => Promise<void>;
}

export const useSourcesStore = create<SourcesState>((set) => {
  const settle = async (
    action: () => Promise<
      { status: "ok"; data: SourceRow[] } | { status: "error"; error: string }
    >,
  ) => {
    set({ busy: true });
    const response = await action();
    if (response.status === "ok") {
      set({ rows: response.data, busy: false, error: null });
      await useScanStore.getState().refresh();
      await useAuditStore.getState().refresh();
    } else {
      set({ busy: false, error: response.error });
    }
  };

  return {
    rows: [],
    busy: false,
    error: null,
    warnings: [],

    load: async () => {
      const response = await commands.sourcesOverview();
      if (response.status === "ok") {
        set({ rows: response.data, error: null });
      } else {
        set({ error: response.error });
      }
    },

    add: (scope, name, reference) =>
      settle(() => commands.sourceAdd(scope, name, reference)),
    remove: (scope, name) => settle(() => commands.sourceRemove(scope, name)),
    toggle: (scope, name, enabled) =>
      settle(() => commands.sourceToggle(scope, name, enabled)),

    refreshRemotes: async () => {
      set({ busy: true });
      const response = await commands.sourcesRefresh();
      if (response.status === "ok") {
        set({ warnings: response.data, busy: false, error: null });
        const fresh = await commands.sourcesOverview();
        if (fresh.status === "ok") set({ rows: fresh.data });
      } else {
        set({ busy: false, error: response.error });
      }
    },
  };
});
