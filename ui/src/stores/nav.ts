import { create } from "zustand";
import type { HarnessId, ItemKind } from "@/bindings";
import type { ScopeSelection } from "@/lib/derive";

export type Page =
  | "home"
  | "review"
  | "library"
  | "tools"
  | "customize"
  | "settings";

/** Which half of the Library page is showing. */
export type LibraryTab = "installed" | "add";

/** Which half of the Tools & Projects page is showing. */
export type ToolsTab = "tools" | "projects";

/** What Library's Installed view should filter to when it first opens. */
export interface LibraryFilter {
  tool?: HarnessId;
  kind?: ItemKind;
}

interface NavState {
  page: Page;
  scope: ScopeSelection;
  libraryTab: LibraryTab;
  toolsTab: ToolsTab;
  /** Consumed once by Installed on mount, then cleared. */
  libraryFilter: LibraryFilter | null;
  setPage: (page: Page) => void;
  setScope: (scope: ScopeSelection) => void;
  goToLibrary: (opts?: { tab?: LibraryTab } & LibraryFilter) => void;
  goToTools: (tab: ToolsTab) => void;
  clearLibraryFilter: () => void;
}

export const useNavStore = create<NavState>((set) => ({
  page: "home",
  scope: "all",
  libraryTab: "installed",
  toolsTab: "tools",
  libraryFilter: null,
  setPage: (page) => set({ page }),
  setScope: (scope) => set({ scope }),
  goToLibrary: ({ tab = "installed", tool, kind } = {}) =>
    set({
      page: "library",
      libraryTab: tab,
      libraryFilter: tool || kind ? { tool, kind } : null,
    }),
  goToTools: (tab) => set({ page: "tools", toolsTab: tab }),
  clearLibraryFilter: () => set({ libraryFilter: null }),
}));
