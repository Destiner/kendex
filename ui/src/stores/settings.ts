import { toast } from "sonner";
import { create } from "zustand";
import {
  type Appearance,
  type AppSettings,
  type CapabilityRow,
  commands,
} from "@/bindings";
import { useScanStore } from "./scan";

interface SettingsState {
  settings: AppSettings | null;
  capabilities: CapabilityRow[];
  load: () => Promise<void>;
  setAppearance: (appearance: Appearance) => Promise<void>;
  setSafety: (warnBelow: number, blockBelow: number) => Promise<void>;
  setHarnessRoot: (harness: string, root: string) => Promise<void>;
  registerProject: (path: string) => Promise<boolean>;
  unregisterProject: (path: string) => Promise<void>;
  discoverProjects: (root: string) => Promise<string[]>;
}

async function rescan() {
  await useScanStore.getState().refresh();
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  capabilities: [],

  load: async () => {
    const [settings, capabilities] = await Promise.all([
      commands.getSettings(),
      commands.capabilityTable(),
    ]);
    if (settings.status === "ok") {
      set({ settings: settings.data, capabilities });
    } else {
      toast.error(settings.error);
    }
  },

  // Theme, safety threshold, and tool folder saves are instant and their
  // effect is visible immediately on screen — a toast on top would just be
  // noise, so success here stays silent and only failure speaks up.
  setAppearance: async (appearance) => {
    const current = get().settings;
    if (!current) return;
    const response = await commands.updateSettings({ ...current, appearance });
    if (response.status === "ok") set({ settings: response.data });
    else toast.error(response.error);
  },

  setSafety: async (warnBelow, blockBelow) => {
    const current = get().settings;
    if (!current) return;
    const response = await commands.updateSettings({
      ...current,
      safety: { "warn-below": warnBelow, "block-below": blockBelow },
    });
    if (response.status === "ok") set({ settings: response.data });
    else toast.error(response.error);
  },

  setHarnessRoot: async (harness, root) => {
    const current = get().settings;
    if (!current) return;
    const roots = { ...current["harness-roots"] };
    if (root.trim() === "") delete roots[harness];
    else roots[harness] = root;
    const response = await commands.updateSettings({
      ...current,
      "harness-roots": roots,
    });
    if (response.status === "ok") {
      set({ settings: response.data });
      await rescan();
    } else {
      toast.error(response.error);
    }
  },

  registerProject: async (path) => {
    const response = await commands.registerProject(path);
    if (response.status === "ok") {
      set({ settings: response.data });
      toast.success(`Added ${path.split("/").pop()}`);
      await rescan();
      return true;
    }
    toast.error(response.error);
    return false;
  },

  unregisterProject: async (path) => {
    const response = await commands.unregisterProject(path);
    if (response.status === "ok") {
      set({ settings: response.data });
      await rescan();
    } else {
      toast.error(response.error);
    }
  },

  discoverProjects: async (root) => {
    const response = await commands.discoverProjects(root);
    if (response.status === "ok") return response.data;
    toast.error(response.error);
    return [];
  },
}));
