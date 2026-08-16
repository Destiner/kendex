import { toast } from "sonner";
import { create } from "zustand";
import {
  type AuditView,
  commands,
  type HarnessId,
  type ItemKind,
  type Scope,
} from "@/bindings";
import { adoptedToastLabel } from "@/lib/copy";
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
  /** Unix ms of the last audit that came back clean; null until one has. */
  auditedAt: number | null;
  refresh: (opts?: { force?: boolean }) => Promise<void>;
  applyPlan: (
    scope: Scope,
    removeOrphans: boolean,
    allowUnsafe?: string[],
  ) => Promise<void>;
  adopt: (
    scope: Scope,
    kind: ItemKind,
    name: string,
    harness: HarnessId,
    opts?: { silent?: boolean },
  ) => Promise<void>;
  toggle: (
    scope: Scope,
    kind: ItemKind,
    name: string,
    enabled: boolean,
  ) => Promise<void>;
  removeItem: (scope: Scope, kind: ItemKind, name: string) => Promise<void>;
}

/** How long an audit answers for before a visit pays for a fresh one. */
const AUDIT_FRESH_FOR_MS = 60_000;

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
    let response: Awaited<ReturnType<typeof action>>;
    try {
      response = await action();
    } finally {
      set({ busy: false });
    }
    if (response.status === "ok") {
      set({ views: replaceView(get().views, response.data), error: null });
      if (opts.successMessage) toast.success(opts.successMessage);
      await useScanStore.getState().refresh();
    } else {
      set({ error: response.error });
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
    auditedAt: null,
    auditing: false,
    error: null,
    busy: false,
    backgroundFailureAnnounced: false,

    // Every visit to Review used to re-audit the whole machine, which is
    // seconds of work to answer a question already on screen. A recent
    // answer is reused; anything the app itself changes refreshes the scope
    // it changed, and a stale window closes on its own inside a minute.
    refresh: async (opts) => {
      if (get().auditing) return;
      const auditedAt = get().auditedAt;
      const fresh =
        auditedAt != null && Date.now() - auditedAt < AUDIT_FRESH_FOR_MS;
      if (fresh && !opts?.force) return;
      set({ auditing: true });
      try {
        const response = await commands.auditAll();
        if (response.status === "ok") {
          set({
            views: response.data,
            auditedAt: Date.now(),
            error: null,
            backgroundFailureAnnounced: false,
          });
        } else {
          set({ error: response.error });
          if (!get().backgroundFailureAnnounced) {
            toast.error(response.error);
            set({ backgroundFailureAnnounced: true });
          }
        }
      } finally {
        set({ auditing: false });
      }
    },

    applyPlan: (scope, removeOrphans, allowUnsafe = []) =>
      run(() => commands.applyPlan(scope, removeOrphans, allowUnsafe), {
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
    toggle: (scope, kind, name, enabled) =>
      run(() => commands.toggleItem(scope, kind, name, enabled), {
        title: `Couldn't ${enabled ? "turn on" : "turn off"} ${name}`,
        steps: ["Try again"],
      }),
    removeItem: (scope, kind, name) =>
      run(() => commands.removeItem(scope, kind, name), {
        title: `Couldn't remove ${name}`,
        steps: ["Try again"],
      }),
  };
});
