import { create } from "zustand";
import { commands, type ScanResult } from "@/bindings";

interface ScanState {
  result: ScanResult | null;
  scanning: boolean;
  error: string | null;
  /** When the last successful scan finished, for the status footer's
   * "scanned Nm ago" — null until the first scan completes. */
  lastScanAt: number | null;
  refresh: () => Promise<void>;
}

export const useScanStore = create<ScanState>((set, get) => ({
  result: null,
  scanning: false,
  error: null,
  lastScanAt: null,
  refresh: async () => {
    if (get().scanning) return;
    set({ scanning: true });
    const response = await commands.scanMachine();
    if (response.status === "ok") {
      set({
        result: response.data,
        scanning: false,
        error: null,
        lastScanAt: Date.now(),
      });
    } else {
      set({ scanning: false, error: response.error });
    }
  },
}));
