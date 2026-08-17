import { toast } from "sonner";
import { create } from "zustand";
import { commands, type ItemWarning, type UpdateRow } from "@/bindings";
import {
  UPDATE_ERROR_TITLE,
  UPDATED_ALL_TOAST,
  updatedToastLabel,
} from "@/lib/copy";
import { scopeKey } from "@/lib/scope";
import { useAuditStore } from "./audit";
import { useProblemsStore } from "./problems";
import { useScanStore } from "./scan";

/** The sidebar badge's number: packages with an update someone would want
 *  to hear about. Ignored ones asked not to be counted; held ones still
 *  count — a hold is "not yet", not "never tell me". */
export const visibleUpdateCount = (rows: UpdateRow[]): number =>
  rows.filter((row) => row.updateAvailable && !row.ignored).length;

/** The Updates page's main list: everything with an update that has not
 *  been muted. */
export const visibleUpdates = (rows: UpdateRow[]): UpdateRow[] =>
  rows.filter((row) => row.updateAvailable && !row.ignored);

/** The collapsed "hidden updates" section: muted packages whose update is
 *  still real — with the way back out. */
export const hiddenUpdates = (rows: UpdateRow[]): UpdateRow[] =>
  rows.filter((row) => row.updateAvailable && row.ignored);

interface UpdatesState {
  rows: UpdateRow[];
  /** Packages whose standing could not be computed — shown, never treated
   *  as current. */
  warnings: ItemWarning[];
  busy: boolean;
  /** True while a mirror fetch is running — the explicit "check". */
  checking: boolean;
  loaded: boolean;
  load: () => Promise<void>;
  check: () => Promise<void>;
  updateOne: (row: UpdateRow) => Promise<void>;
  updateAll: () => Promise<void>;
  setAutoUpdate: (row: UpdateRow, auto: boolean) => Promise<void>;
  setIgnored: (row: UpdateRow, ignored: boolean) => Promise<void>;
}

export const useUpdatesStore = create<UpdatesState>((set, get) => {
  const showError = (title: string, message: string) =>
    useProblemsStore.getState().showError({ title, message });

  const apply = async (row: UpdateRow): Promise<boolean> => {
    // Held packages move by moving the hold; following ones come current
    // by applying the scope — which is what following means, and brings
    // any other pending changes in that scope along.
    const response =
      row.pinned && row.latest
        ? await commands.packageSetRev(
            row.scope,
            row.kind,
            row.name,
            row.latest.commit,
          )
        : await commands.applyPlan(row.scope, false, []);
    if (response.status === "error") {
      showError(UPDATE_ERROR_TITLE, response.error);
      return false;
    }
    return true;
  };

  const reload = async () => {
    const response = await commands.updatesOverview();
    // A failed reload marks the data stale (loaded = false) rather than
    // leaving the last-good rows trusted — the package page gates the
    // Update button on `loaded`, and acting on rows we could not refresh
    // is exactly the fail-open this closes.
    if (response.status === "ok")
      set({
        rows: response.data.rows,
        warnings: response.data.warnings,
        loaded: true,
      });
    else set({ loaded: false });
  };

  return {
    rows: [],
    warnings: [],
    busy: false,
    checking: false,
    loaded: false,

    load: async () => {
      await reload();
    },

    check: async () => {
      set({ checking: true });
      try {
        const response = await commands.updatesRefresh();
        if (response.status === "ok") {
          set({
            rows: response.data.rows,
            warnings: response.data.warnings,
            loaded: true,
          });
        } else {
          set({ loaded: false });
          showError(UPDATE_ERROR_TITLE, response.error);
        }
      } finally {
        set({ checking: false });
      }
    },

    updateOne: async (row) => {
      set({ busy: true });
      try {
        if (await apply(row)) {
          toast.success(updatedToastLabel(row.name));
          await reload();
          await useScanStore.getState().refresh();
          await useAuditStore.getState().refresh({ force: true });
        }
      } finally {
        set({ busy: false });
      }
    },

    updateAll: async () => {
      set({ busy: true });
      try {
        // Edited packages are held by the engine and cannot be updated
        // this way — they need the fork decision first, so they are left
        // out of "update all" rather than silently surviving it.
        const rows = visibleUpdates(get().rows).filter(
          (row) => !row.blockedByLocalEdit,
        );
        // Move every hold first, then one apply per scope brings the
        // followers current — never two applies for one scope.
        let ok = true;
        for (const row of rows.filter((row) => row.pinned)) {
          ok = (await apply(row)) && ok;
        }
        const scopes = new Map(
          rows
            .filter((row) => !row.pinned)
            .map((row) => [scopeKey(row.scope), row] as const),
        );
        for (const row of scopes.values()) {
          const response = await commands.applyPlan(row.scope, false, []);
          if (response.status === "error") {
            showError(UPDATE_ERROR_TITLE, response.error);
            ok = false;
          }
        }
        if (ok) toast.success(UPDATED_ALL_TOAST);
        await reload();
        await useScanStore.getState().refresh();
        await useAuditStore.getState().refresh({ force: true });
      } finally {
        set({ busy: false });
      }
    },

    setAutoUpdate: async (row, auto) => {
      // Turning automatic OFF holds the package at what is installed now.
      // With nothing installed to hold at, there is nothing to switch —
      // never fall through to null, which means "follow" (the opposite).
      const hold = row.current?.commit ?? null;
      if (!auto && hold === null) return;
      set({ busy: true });
      try {
        const response = await commands.packageSetRev(
          row.scope,
          row.kind,
          row.name,
          auto ? null : hold,
        );
        if (response.status === "error") {
          showError(UPDATE_ERROR_TITLE, response.error);
        }
        await reload();
      } finally {
        set({ busy: false });
      }
    },

    setIgnored: async (row, ignored) => {
      const response = await commands.updateSetIgnored(
        row.scope,
        row.kind,
        row.name,
        row.repo,
        ignored,
      );
      if (response.status === "ok")
        set({
          rows: response.data.rows,
          warnings: response.data.warnings,
          loaded: true,
        });
      else showError(UPDATE_ERROR_TITLE, response.error);
    },
  };
});
