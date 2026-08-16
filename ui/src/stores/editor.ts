import { create } from "zustand";
import { commands, type EditorInventory, type Scope } from "@/bindings";
import { type Draft, emptyDraft, toDraft } from "@/lib/editor-draft";
import { useAuditStore } from "./audit";
import { useScanStore } from "./scan";

interface EditorState {
  /** The single scope being edited — deliberately not the sidebar filter. */
  scope: Scope;
  draft: Draft | null;
  inventory: EditorInventory | null;
  /** No manifest at this scope yet; the page offers to create one. */
  absent: boolean;
  dirty: boolean;
  loading: boolean;
  saving: boolean;
  error: string | null;
  setScope: (scope: Scope) => Promise<void>;
  load: () => Promise<void>;
  edit: (change: (draft: Draft) => Draft) => void;
  save: () => Promise<void>;
  create: () => Promise<void>;
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
      set({ draft: null, absent: false, dirty: false, error: manifest.error });
      return;
    }
    set({
      draft: manifest.data ? toDraft(manifest.data) : null,
      absent: manifest.data === null,
      inventory: inventory.status === "ok" ? inventory.data : get().inventory,
      dirty: false,
      error: inventory.status === "ok" ? null : inventory.error,
    });
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
    absent: false,
    dirty: false,
    loading: false,
    saving: false,
    error: null,

    setScope: async (scope) => {
      set({ scope, draft: null, dirty: false, error: null });
      await load();
    },

    load,

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

    create: () => write(emptyDraft()),
  };
});
