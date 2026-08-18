import { create } from "zustand";
import { commands, type EditorInventory, type Scope } from "@/bindings";
import { type Draft, emptyDraft, toDraft } from "@/lib/editor-draft";
import { sameScope, scopeKey } from "@/lib/scope";
import { useAuditStore } from "./audit";
import { useScanStore } from "./scan";
import { useSettingsStore } from "./settings";

interface EditorState {
  /** The single scope being edited — deliberately not the sidebar filter. */
  scope: Scope;
  draft: Draft | null;
  inventory: EditorInventory | null;
  /** Every scope's saved manifest, keyed by scope. What the Library and the
   *  Customize index read to mark what has been customized; `draft` above is
   *  the one copy being edited. */
  saved: Record<string, Draft>;
  dirty: boolean;
  loading: boolean;
  saving: boolean;
  error: string | null;
  setScope: (scope: Scope) => Promise<void>;
  /** Point the editor at a scope without discarding edits already in hand. */
  openScope: (scope: Scope) => Promise<void>;
  load: () => Promise<void>;
  /** Read every scope's manifest, for the marks drawn outside the editor. */
  loadAll: () => Promise<void>;
  edit: (change: (draft: Draft) => Draft) => void;
  save: () => Promise<void>;
}

export const useEditorStore = create<EditorState>((set, get) => {
  const load = async () => {
    const { scope } = get();
    set({ loading: true });
    let manifest: Awaited<ReturnType<typeof commands.getManifest>>;
    let inventory: Awaited<ReturnType<typeof commands.editorInventory>>;
    try {
      [manifest, inventory] = await Promise.all([
        commands.getManifest(scope),
        commands.editorInventory(scope),
      ]);
    } finally {
      set({ loading: false });
    }
    if (manifest.status === "error") {
      set({ draft: null, dirty: false, error: manifest.error });
      return;
    }
    // With no manifest here yet the editor still opens, on an empty one:
    // asking someone to press "create" before they can type is a step that
    // decides nothing. Saving is what writes the file.
    const draft = manifest.data ? toDraft(manifest.data) : emptyDraft();
    set((state) => ({
      draft,
      inventory: inventory.status === "ok" ? inventory.data : state.inventory,
      saved: { ...state.saved, [scopeKey(scope)]: draft },
      dirty: false,
      error: inventory.status === "ok" ? null : inventory.error,
    }));
  };

  const write = async (draft: Draft) => {
    const { scope } = get();
    set({ saving: true });
    let response: Awaited<ReturnType<typeof commands.updateManifest>>;
    try {
      response = await commands.updateManifest(scope, draft);
    } finally {
      set({ saving: false });
    }
    if (response.status === "error") {
      set({ error: response.error });
      return;
    }
    set({ error: null });
    await load();
    await useAuditStore.getState().refresh();
    await useScanStore.getState().refresh();
  };

  return {
    scope: { scope: "global" },
    draft: null,
    inventory: null,
    saved: {},
    dirty: false,
    loading: false,
    saving: false,
    error: null,

    setScope: async (scope) => {
      set({ scope, draft: null, dirty: false, error: null });
      await load();
    },

    openScope: async (scope) => {
      const state = get();
      if (state.draft && sameScope(state.scope, scope)) return;
      await state.setScope(scope);
    },

    load,

    loadAll: async () => {
      const projects = useSettingsStore.getState().settings?.projects ?? [];
      const scopes: Scope[] = [
        { scope: "global" },
        ...projects.map((root) => ({ scope: "project" as const, root })),
      ];
      const loaded = await Promise.all(
        scopes.map((scope) => commands.getManifest(scope)),
      );
      const saved: Record<string, Draft> = {};
      for (const [index, response] of loaded.entries()) {
        if (response.status !== "ok") continue;
        saved[scopeKey(scopes[index])] = response.data
          ? toDraft(response.data)
          : emptyDraft();
      }
      set({ saved });
    },

    edit: (change) => {
      const { draft } = get();
      if (!draft) return;
      set({ draft: change(draft), dirty: true });
    },

    save: async () => {
      const { draft } = get();
      if (!draft) return;
      await write(draft);
    },
  };
});
