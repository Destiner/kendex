import { create } from "zustand";
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

interface NavState {
  page: Page;
  scope: ScopeSelection;
  libraryTab: LibraryTab;
  toolsTab: ToolsTab;
  setPage: (page: Page) => void;
  setScope: (scope: ScopeSelection) => void;
  goToLibrary: (tab: LibraryTab) => void;
  goToTools: (tab: ToolsTab) => void;
}

export const useNavStore = create<NavState>((set) => ({
  page: "home",
  scope: "all",
  libraryTab: "installed",
  toolsTab: "tools",
  setPage: (page) => set({ page }),
  setScope: (scope) => set({ scope }),
  goToLibrary: (tab) => set({ page: "library", libraryTab: tab }),
  goToTools: (tab) => set({ page: "tools", toolsTab: tab }),
}));
