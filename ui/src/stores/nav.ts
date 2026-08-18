import { create } from "zustand";
import type { HarnessId, ItemKind, Scope } from "@/bindings";
import type { ScopeSelection } from "@/lib/derive";

export type Page =
  | "home"
  | "review"
  | "library"
  | "harnesses"
  | "projects"
  | "customize"
  // Reached from Home's attention list and the Review card's footnote —
  // adopting is an offer, not a sidebar destination.
  | "unmanaged"
  | "settings"
  | "updates"
  // Reached only from the status footer's problems segment or a review
  // card's "See all problems" — not in the sidebar, since it isn't a place
  // you'd navigate to when nothing is wrong.
  | "problems"
  // Reached only by opening a package from a list — which package is open
  // lives in `packageRef`, so the page is never a sidebar destination.
  | "package";

/** Which half of the Library page is showing. */
export type LibraryTab = "installed" | "add";

/** What Library's Installed view should filter to when it first opens. */
export interface LibraryFilter {
  harness?: HarnessId;
  kind?: ItemKind;
}

/** The package a package page is showing — everything a backend query
 * needs to address it. */
export interface PackageRef {
  kind: ItemKind;
  name: string;
  scope: Scope;
}

/** What the package page should open showing, when not its files — e.g.
 * "Preview" on the Updates page lands straight on the diff. Consumed once
 * by the page on mount, then cleared. */
export interface PackageView {
  mode: "diff";
  from: string;
  to: string;
}

/** Where the back button returns to: a page plus its tab state at push time. */
export interface HistoryEntry {
  page: Page;
  libraryTab: LibraryTab;
  packageRef: PackageRef | null;
}

// Small and fixed so a long session of cross-page hops never grows the
// stack unbounded — nobody needs to back up more than this in practice.
const HISTORY_CAP = 20;

interface NavState {
  page: Page;
  /** Which locations the Library's table shows. It belongs to that page —
   *  every other page states the location on each row instead, so nothing
   *  is ever hidden behind a filter set somewhere else. */
  libraryScope: ScopeSelection;
  /** What the Library's search box holds, kept here so leaving the page and
   * coming back keeps the table narrowed the same way. */
  search: string;
  /** Bumped whenever something asks for the search box. The box focuses
   * itself on every change, so the "/" shortcut reaches it from any page
   * rather than only from the one it happens to be mounted on. */
  searchFocus: number;
  libraryTab: LibraryTab;
  /** Consumed once by Installed on mount, then cleared. */
  libraryFilter: LibraryFilter | null;
  /** Which package the package page shows; null anywhere else. */
  packageRef: PackageRef | null;
  /** Consumed once by the package page on mount, then cleared. */
  packageView: PackageView | null;
  history: HistoryEntry[];
  /** Where back has been from, newest last — the other half of a browser's
   * pair. Any fresh navigation abandons it, exactly as a browser does. */
  future: HistoryEntry[];
  setPage: (page: Page) => void;
  setLibraryScope: (scope: ScopeSelection) => void;
  setSearch: (search: string) => void;
  /** Send the user to the Library with the cursor in its search box. */
  focusSearch: () => void;
  goToLibrary: (opts?: { tab?: LibraryTab } & LibraryFilter) => void;
  /** A cross-page link from chrome that's always on screen (e.g. the status
   * footer) — pushes history like the other goTo* helpers so back and the
   * breadcrumb work, without needing per-tab state of its own. */
  goTo: (page: Page) => void;
  goToPackage: (ref: PackageRef, view?: PackageView) => void;
  clearLibraryFilter: () => void;
  clearPackageView: () => void;
  back: () => void;
  forward: () => void;
}

export const useNavStore = create<NavState>((set) => ({
  page: "home",
  libraryScope: "all",
  search: "",
  searchFocus: 0,
  libraryTab: "installed",
  libraryFilter: null,
  packageRef: null,
  packageView: null,
  history: [],
  future: [],
  // A direct page pick starts a fresh navigation context — an old back
  // trail pointing at a page the user deliberately left is a bug, not a
  // shortcut, and a stale filter from before the jump shouldn't resurface.
  setPage: (page) =>
    set({
      page,
      history: [],
      future: [],
      libraryFilter: null,
      packageRef: null,
    }),
  setLibraryScope: (libraryScope) => set({ libraryScope }),
  setSearch: (search) => set({ search }),
  // Asking to search means "find me this thing", and the only page that can
  // answer is the Library — so the shortcut takes you there rather than
  // putting a cursor in a box that filters a list you cannot see.
  focusSearch: () =>
    set((state) => ({
      page: "library",
      libraryTab: "installed",
      searchFocus: state.searchFocus + 1,
      history:
        state.page === "library"
          ? state.history
          : pushHistory(state, "library"),
      future: state.page === "library" ? state.future : [],
    })),
  goToLibrary: ({ tab = "installed", harness, kind } = {}) =>
    set((state) => ({
      page: "library",
      libraryTab: tab,
      libraryFilter: harness || kind ? { harness, kind } : null,
      history: pushHistory(state, "library"),
      future: [],
    })),
  goTo: (page) =>
    set((state) => ({
      page,
      history: pushHistory(state, page),
      future: [],
    })),
  goToPackage: (ref, view) =>
    set((state) => ({
      page: "package",
      packageRef: ref,
      packageView: view ?? null,
      history: pushHistory(state, "package"),
      future: [],
    })),
  clearLibraryFilter: () => set({ libraryFilter: null }),
  clearPackageView: () => set({ packageView: null }),
  back: () =>
    set((state) => {
      const prior = state.history.at(-1);
      if (!prior) return state;
      return {
        ...prior,
        libraryFilter: null,
        packageView: null,
        history: state.history.slice(0, -1),
        future: [...state.future, here(state)].slice(-HISTORY_CAP),
      };
    }),
  forward: () =>
    set((state) => {
      const next = state.future.at(-1);
      if (!next) return state;
      return {
        ...next,
        libraryFilter: null,
        packageView: null,
        history: [...state.history, here(state)].slice(-HISTORY_CAP),
        future: state.future.slice(0, -1),
      };
    }),
}));

/** The entry describing where the user is standing right now. */
function here(state: NavState): HistoryEntry {
  return {
    page: state.page,
    libraryTab: state.libraryTab,
    packageRef: state.packageRef,
  };
}

// Only a real page change is worth a stack entry — switching tabs within
// the page you're already on isn't a "place" to come back to.
function pushHistory(state: NavState, destination: Page): HistoryEntry[] {
  if (state.page === destination) return state.history;
  return [...state.history, here(state)].slice(-HISTORY_CAP);
}
