import { toast } from "sonner";
import { create } from "zustand";
import { commands, type UpdateRow } from "@/bindings";
import {
  UPDATE_ERROR_TITLE,
  UPDATED_ALL_TOAST,
  updatedToastLabel,
} from "@/lib/copy";
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
        : await commands.applyPlan(row.scope, false);
    if (response.status === "error") {
      showError(UPDATE_ERROR_TITLE, response.error);
      return false;
    }
    return true;
  };

  const reload = async () => {
    const response = await commands.updatesOverview();
    if (response.status === "ok") set({ rows: response.data, loaded: true });
  };

  return {
    rows: [],
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
          set({ rows: response.data, loaded: true });
        } else {
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
        // Move every hold first, then one apply per scope brings the
        // followers current — never two applies for one scope.
        const rows = visibleUpdates(get().rows);
        let ok = true;
        for (const row of rows.filter((row) => row.pinned)) {
          ok = (await apply(row)) && ok;
        }
        const scopes = new Map(
          rows
            .filter((row) => !row.pinned)
            .map((row) => [JSON.stringify(row.scope), row] as const),
        );
        for (const row of scopes.values()) {
          ok =
            (await commands.applyPlan(row.scope, false)).status === "ok" && ok;
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
      set({ busy: true });
      try {
        const response = await commands.packageSetRev(
          row.scope,
          row.kind,
          row.name,
          auto ? null : (row.current?.commit ?? null),
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
      if (response.status === "ok") set({ rows: response.data, loaded: true });
      else showError(UPDATE_ERROR_TITLE, response.error);
    },
  };
});
