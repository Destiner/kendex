import { create } from "zustand";
import { commands, type ScanResult } from "@/bindings";

interface ScanState {
  result: ScanResult | null;
  scanning: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export const useScanStore = create<ScanState>((set, get) => ({
  result: null,
  scanning: false,
  error: null,
  refresh: async () => {
    if (get().scanning) return;
    set({ scanning: true });
    const response = await commands.scanMachine();
    if (response.status === "ok") {
      set({ result: response.data, scanning: false, error: null });
    } else {
      set({ scanning: false, error: response.error });
    }
  },
}));
