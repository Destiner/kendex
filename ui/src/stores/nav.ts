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

/** Where the back button returns to: a page plus its tab state at push time. */
export interface HistoryEntry {
  page: Page;
  libraryTab: LibraryTab;
  toolsTab: ToolsTab;
}

// Small and fixed so a long session of cross-page hops never grows the
// stack unbounded — nobody needs to back up more than this in practice.
const HISTORY_CAP = 20;

interface NavState {
  page: Page;
  scope: ScopeSelection;
  libraryTab: LibraryTab;
  toolsTab: ToolsTab;
  /** Consumed once by Installed on mount, then cleared. */
  libraryFilter: LibraryFilter | null;
  history: HistoryEntry[];
  setPage: (page: Page) => void;
  setScope: (scope: ScopeSelection) => void;
  goToLibrary: (opts?: { tab?: LibraryTab } & LibraryFilter) => void;
  goToTools: (tab: ToolsTab) => void;
  /** A cross-page link from chrome that's always on screen (e.g. the status
   * footer) — pushes history like the other goTo* helpers so back and the
   * breadcrumb work, without needing per-tab state of its own. */
  goTo: (page: Page) => void;
  clearLibraryFilter: () => void;
  back: () => void;
}

export const useNavStore = create<NavState>((set) => ({
  page: "home",
  scope: "all",
  libraryTab: "installed",
  toolsTab: "tools",
  libraryFilter: null,
  history: [],
  // A direct page pick starts a fresh navigation context — an old back
  // trail pointing at a page the user deliberately left is a bug, not a
  // shortcut, and a stale filter from before the jump shouldn't resurface.
  setPage: (page) => set({ page, history: [], libraryFilter: null }),
  setScope: (scope) => set({ scope }),
  goToLibrary: ({ tab = "installed", tool, kind } = {}) =>
    set((state) => ({
      page: "library",
      libraryTab: tab,
      libraryFilter: tool || kind ? { tool, kind } : null,
      history: pushHistory(state, "library"),
    })),
  goToTools: (tab) =>
    set((state) => ({
      page: "tools",
      toolsTab: tab,
      history: pushHistory(state, "tools"),
    })),
  goTo: (page) =>
    set((state) => ({
      page,
      history: pushHistory(state, page),
    })),
  clearLibraryFilter: () => set({ libraryFilter: null }),
  back: () =>
    set((state) => {
      const prior = state.history.at(-1);
      if (!prior) return state;
      return {
        ...prior,
        libraryFilter: null,
        history: state.history.slice(0, -1),
      };
    }),
}));

// Only a real page change is worth a stack entry — switching tabs within
// the page you're already on isn't a "place" to come back to.
function pushHistory(state: NavState, destination: Page): HistoryEntry[] {
  if (state.page === destination) return state.history;
  const entry: HistoryEntry = {
    page: state.page,
    libraryTab: state.libraryTab,
    toolsTab: state.toolsTab,
  };
  return [...state.history, entry].slice(-HISTORY_CAP);
}
