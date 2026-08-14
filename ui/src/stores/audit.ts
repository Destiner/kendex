import { toast } from "sonner";
import { create } from "zustand";
import {
  type AuditView,
  commands,
  type HarnessId,
  type ItemKind,
  type Scope,
} from "@/bindings";
import { adoptedToastLabel } from "@/lib/labels";
import { useScanStore } from "./scan";

interface AuditState {
  views: AuditView[];
  auditing: boolean;
  error: string | null;
  busy: boolean;
  /** The startup audit has already toasted its failure — suppresses repeat
   * toasts on every silent retry until one succeeds. */
  backgroundFailureAnnounced: boolean;
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
  // A row that vanishes with no word said is indistinguishable from a
  // button that did nothing — every outcome here gets a toast, success or
  // failure, on top of the state update the page renders from.
  const run = async (
    action: () => Promise<
      { status: "ok"; data: AuditView } | { status: "error"; error: string }
    >,
    successMessage?: string,
  ) => {
    set({ busy: true });
    const response = await action();
    if (response.status === "ok") {
      set({
        views: replaceView(get().views, response.data),
        busy: false,
        error: null,
      });
      if (successMessage) toast.success(successMessage);
      await useScanStore.getState().refresh();
    } else {
      set({ busy: false, error: response.error });
      toast.error(response.error);
    }
  };

  return {
    views: [],
    auditing: false,
    error: null,
    busy: false,
    backgroundFailureAnnounced: false,

    refresh: async () => {
      if (get().auditing) return;
      set({ auditing: true });
      const response = await commands.auditAll();
      if (response.status === "ok") {
        set({
          views: response.data,
          auditing: false,
          error: null,
          backgroundFailureAnnounced: false,
        });
      } else {
        set({ auditing: false, error: response.error });
        if (!get().backgroundFailureAnnounced) {
          toast.error(response.error);
          set({ backgroundFailureAnnounced: true });
        }
      }
    },

    applyPlan: (scope, removeOrphans) =>
      run(() => commands.applyPlan(scope, removeOrphans)),
    adopt: (scope, kind, name, harness) =>
      run(
        () => commands.adoptItem(scope, kind, name, harness),
        adoptedToastLabel(name),
      ),
    toggle: (scope, name, enabled) =>
      run(() => commands.toggleItem(scope, name, enabled)),
    removeItem: (scope, name) => run(() => commands.removeItem(scope, name)),
  };
});
