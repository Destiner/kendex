import { create } from "zustand";
import {
  type AuditView,
  commands,
  type HarnessId,
  type ItemKind,
  type Scope,
} from "@/bindings";
import { useScanStore } from "./scan";

interface AuditState {
  views: AuditView[];
  auditing: boolean;
  error: string | null;
  busy: boolean;
  refresh: () => Promise<void>;
  applyPlan: (scope: Scope, removeOrphans: boolean) => Promise<void>;
  adopt: (
    scope: Scope,
    kind: ItemKind,
    name: string,
    harness: HarnessId,
  ) => Promise<void>;
  toggle: (scope: Scope, name: string, enabled: boolean) => Promise<void>;
  removeItem: (scope: Scope, name: string) => Promise<void>;
}

function replaceView(views: AuditView[], fresh: AuditView): AuditView[] {
  return views.map((view) =>
    sameScope(view.scope, fresh.scope) ? fresh : view,
  );
}

export function sameScope(a: Scope, b: Scope): boolean {
  if (a.scope === "global" && b.scope === "global") return true;
  return a.scope === "project" && b.scope === "project" && a.root === b.root;
}

export const useAuditStore = create<AuditState>((set, get) => {
  const run = async (
    action: () => Promise<
      { status: "ok"; data: AuditView } | { status: "error"; error: string }
    >,
  ) => {
    set({ busy: true });
    const response = await action();
    if (response.status === "ok") {
      set({
        views: replaceView(get().views, response.data),
        busy: false,
        error: null,
      });
      await useScanStore.getState().refresh();
    } else {
      set({ busy: false, error: response.error });
    }
  };

  return {
    views: [],
    auditing: false,
    error: null,
    busy: false,

    refresh: async () => {
      if (get().auditing) return;
      set({ auditing: true });
      const response = await commands.auditAll();
      if (response.status === "ok") {
        set({ views: response.data, auditing: false, error: null });
      } else {
        set({ auditing: false, error: response.error });
      }
    },

    applyPlan: (scope, removeOrphans) =>
      run(() => commands.applyPlan(scope, removeOrphans)),
    adopt: (scope, kind, name, harness) =>
      run(() => commands.adoptItem(scope, kind, name, harness)),
    toggle: (scope, name, enabled) =>
      run(() => commands.toggleItem(scope, name, enabled)),
    removeItem: (scope, name) => run(() => commands.removeItem(scope, name)),
  };
});
