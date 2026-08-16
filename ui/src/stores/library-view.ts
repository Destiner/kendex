import { create } from "zustand";
import type { Location } from "@/lib/derive";

/** The Installed table's view state, kept outside the component so opening
 * a package (a real page, which unmounts the table) and coming back lands
 * on the same filters and the same scroll position. Session-lifetime only —
 * a fresh launch starts clean, like every other filter in the app. Values
 * use the filter strip's own vocabulary: "any" means unfiltered. */
interface LibraryViewState {
  kind: string;
  harness: string;
  tag: string;
  locations: Set<Location>;
  /** The table's scroll offset when it last unmounted. */
  scrollTop: number;
  setKind: (kind: string) => void;
  setHarness: (harness: string) => void;
  setTag: (tag: string) => void;
  setLocations: (locations: Set<Location>) => void;
  setScrollTop: (scrollTop: number) => void;
  clearFilters: () => void;
}

export const useLibraryViewStore = create<LibraryViewState>((set) => ({
  kind: "any",
  harness: "any",
  tag: "any",
  locations: new Set<Location>(),
  scrollTop: 0,
  setKind: (kind) => set({ kind }),
  setHarness: (harness) => set({ harness }),
  setTag: (tag) => set({ tag }),
  setLocations: (locations) => set({ locations }),
  setScrollTop: (scrollTop) => set({ scrollTop }),
  clearFilters: () =>
    set({ kind: "any", harness: "any", tag: "any", locations: new Set() }),
}));
