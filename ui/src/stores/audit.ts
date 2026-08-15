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
import { type ErrorAction, useProblemsStore } from "./problems";
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
    opts?: { silent?: boolean },
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
  // button that did nothing — every outcome here speaks up, success or
  // failure, on top of the state update the page renders from. Failure is a
  // modal, not a toast: these are all user-initiated, so the user is looking
  // right at the button that just broke.
  const run = async (
    action: () => Promise<
      { status: "ok"; data: AuditView } | { status: "error"; error: string }
    >,
    opts: { title: string; successMessage?: string; steps?: string[] },
  ) => {
    set({ busy: true });
    const response = await action();
    if (response.status === "ok") {
      set({
        views: replaceView(get().views, response.data),
        busy: false,
        error: null,
      });
      if (opts.successMessage) toast.success(opts.successMessage);
      await useScanStore.getState().refresh();
    } else {
      set({ busy: false, error: response.error });
      const retry: ErrorAction = {
        label: "Retry",
        onClick: () => void run(action, opts),
      };
      useProblemsStore.getState().showError({
        title: opts.title,
        message: response.error,
        steps: opts.steps,
        actions: [retry],
      });
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
      run(() => commands.applyPlan(scope, removeOrphans), {
        title: "Couldn't apply these changes",
        steps: [
          "Nothing was changed — try again",
          "If it keeps failing, check the project folder is writable",
        ],
      }),
    // A merged row adopts every one of its installations in one click —
    // each is its own backend call, but they're one thing to the user, so
    // only the first speaks up with a toast.
    adopt: (scope, kind, name, harness, opts) =>
      run(() => commands.adoptItem(scope, kind, name, harness), {
        title: `Couldn't start managing ${name}`,
        successMessage: opts?.silent ? undefined : adoptedToastLabel(name),
        steps: ["Try again"],
      }),
    toggle: (scope, name, enabled) =>
      run(() => commands.toggleItem(scope, name, enabled), {
        title: `Couldn't ${enabled ? "turn on" : "turn off"} ${name}`,
        steps: ["Try again"],
      }),
    removeItem: (scope, name) =>
      run(() => commands.removeItem(scope, name), {
        title: `Couldn't remove ${name}`,
        steps: ["Try again"],
      }),
  };
});
