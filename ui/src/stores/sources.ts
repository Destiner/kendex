import { toast } from "sonner";
import { create } from "zustand";
import {
  type BundleRow,
  commands,
  type Scope,
  type SourceRow,
} from "@/bindings";
import { installedAutoToastLabel, UPDATE_MANUALLY_ACTION } from "@/lib/copy";
import { useAuditStore } from "./audit";
import { useScanStore } from "./scan";
import { useUpdatesStore } from "./updates";

interface SourcesState {
  rows: SourceRow[];
  bundles: BundleRow[];
  busy: boolean;
  error: string | null;
  warnings: string[];
  load: () => Promise<void>;
  add: (scope: Scope, name: string, reference: string) => Promise<void>;
  remove: (scope: Scope, name: string) => Promise<void>;
  toggle: (scope: Scope, name: string, enabled: boolean) => Promise<void>;
  installBundle: (scope: Scope, source: string, name: string) => Promise<void>;
  refreshRemotes: () => Promise<void>;
}

export const useSourcesStore = create<SourcesState>((set) => {
  const settle = async (
    action: () => Promise<
      { status: "ok"; data: SourceRow[] } | { status: "error"; error: string }
    >,
  ) => {
    set({ busy: true });
    let response: Awaited<ReturnType<typeof action>>;
    try {
      response = await action();
    } finally {
      set({ busy: false });
    }
    if (response.status === "ok") {
      set({ rows: response.data, error: null });
      await useScanStore.getState().refresh();
      await useAuditStore.getState().refresh();
    } else {
      set({ error: response.error });
    }
  };

  const loadBundles = async () => {
    const response = await commands.bundlesOverview();
    if (response.status === "ok") set({ bundles: response.data });
  };

  return {
    rows: [],
    bundles: [],
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
      await loadBundles();
    },

    add: (scope, name, reference) =>
      settle(() => commands.sourceAdd(scope, name, reference)).then(
        loadBundles,
      ),
    remove: (scope, name) =>
      settle(() => commands.sourceRemove(scope, name)).then(loadBundles),
    toggle: (scope, name, enabled) =>
      settle(() => commands.sourceToggle(scope, name, enabled)).then(
        loadBundles,
      ),

    installBundle: async (scope, source, name) => {
      set({ busy: true });
      let response: Awaited<ReturnType<typeof commands.bundleInstall>>;
      try {
        response = await commands.bundleInstall(scope, source, name, false);
      } finally {
        set({ busy: false });
      }
      if (response.status === "ok") {
        set({ bundles: response.data, error: null });
        await useScanStore.getState().refresh();
        await useAuditStore.getState().refresh();
        void useUpdatesStore.getState().load();
        // The one-time auto-or-manual ask, made at the success moment with
        // a zero-tap default: it keeps itself current unless the user taps
        // the other way, which re-declares the same install held at
        // today's version.
        toast.success(installedAutoToastLabel(name), {
          action: {
            label: UPDATE_MANUALLY_ACTION,
            onClick: () => {
              void commands.bundleInstall(scope, source, name, true);
            },
          },
        });
      } else {
        set({ error: response.error });
      }
    },

    refreshRemotes: async () => {
      set({ busy: true });
      try {
        const response = await commands.sourcesRefresh();
        if (response.status === "ok") {
          set({ warnings: response.data, error: null });
          const fresh = await commands.sourcesOverview();
          if (fresh.status === "ok") set({ rows: fresh.data });
          await loadBundles();
        } else {
          set({ error: response.error });
        }
      } finally {
        set({ busy: false });
      }
    },
  };
});
