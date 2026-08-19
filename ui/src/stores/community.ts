// The Community tab's state: the kendex.ai directory (served from the
// app's on-disk cache, honest about staleness) and skills.sh search.
import { create } from "zustand";
import { commands, type DirectoryView, type SkillsShHit } from "@/bindings";

interface CommunityState {
  directory: DirectoryView | null;
  loading: boolean;
  /** Set only when there is nothing to show at all — a stale list renders
   * with its "as of" line instead. */
  error: string | null;

  skillsshAvailable: boolean;
  skillsshHits: SkillsShHit[] | null;
  skillsshSearching: boolean;
  skillsshError: string | null;

  load: (refresh: boolean) => Promise<void>;
  searchSkillssh: (query: string) => Promise<void>;
}

/** Stale results never land on a newer query. */
let searchGeneration = 0;

export const useCommunityStore = create<CommunityState>((set) => ({
  directory: null,
  loading: false,
  error: null,
  skillsshAvailable: true,
  skillsshHits: null,
  skillsshSearching: false,
  skillsshError: null,

  load: async (refresh) => {
    set({ loading: true });
    try {
      const [view, available] = await Promise.all([
        commands.communityDirectory(refresh),
        commands.communitySkillsshAvailable(),
      ]);
      if (view.status === "ok") {
        set({ directory: view.data, error: null });
      } else {
        set({ error: view.error });
      }
      set({
        skillsshAvailable: available.status === "ok" ? available.data : false,
      });
    } finally {
      set({ loading: false });
    }
  },

  searchSkillssh: async (query) => {
    const generation = ++searchGeneration;
    if (!query.trim()) {
      set({
        skillsshHits: null,
        skillsshError: null,
        skillsshSearching: false,
      });
      return;
    }
    set({ skillsshSearching: true });
    const response = await commands.communitySkillsshSearch(query);
    if (generation !== searchGeneration) return;
    if (response.status === "ok") {
      set({
        skillsshHits: response.data,
        skillsshError: null,
        skillsshSearching: false,
      });
    } else {
      set({ skillsshError: response.error, skillsshSearching: false });
    }
  },
}));
