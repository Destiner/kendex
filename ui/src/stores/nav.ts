import { create } from "zustand";
import type { ScopeSelection } from "@/lib/derive";

export type Page =
  | "overview"
  | "items"
  | "harnesses"
  | "scopes"
  | "audit"
  | "settings";

interface NavState {
  page: Page;
  scope: ScopeSelection;
  setPage: (page: Page) => void;
  setScope: (scope: ScopeSelection) => void;
}

export const useNavStore = create<NavState>((set) => ({
  page: "overview",
  scope: "all",
  setPage: (page) => set({ page }),
  setScope: (scope) => set({ scope }),
}));
