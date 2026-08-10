import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ScanResult } from "@/bindings";
import { commands } from "@/bindings";
import { useScanStore } from "./scan";

vi.mock("@/bindings", () => ({
  commands: {
    scanMachine: vi.fn(),
  },
}));

const emptyResult: ScanResult = {
  harnesses: [],
  items: [],
  missingProjects: [],
  warnings: [],
};

describe("scan store", () => {
  beforeEach(() => {
    useScanStore.setState({ result: null, scanning: false, error: null });
    vi.clearAllMocks();
  });

  it("stores the result on success and clears prior errors", async () => {
    useScanStore.setState({ error: "old failure" });
    vi.mocked(commands.scanMachine).mockResolvedValue({
      status: "ok",
      data: emptyResult,
    });

    await useScanStore.getState().refresh();

    const state = useScanStore.getState();
    expect(state.result).toEqual(emptyResult);
    expect(state.error).toBeNull();
    expect(state.scanning).toBe(false);
  });

  it("keeps the last good result when a rescan fails", async () => {
    useScanStore.setState({ result: emptyResult });
    vi.mocked(commands.scanMachine).mockResolvedValue({
      status: "error",
      error: "boom",
    });

    await useScanStore.getState().refresh();

    const state = useScanStore.getState();
    expect(state.result).toEqual(emptyResult);
    expect(state.error).toBe("boom");
  });

  it("ignores refresh while a scan is already running", async () => {
    useScanStore.setState({ scanning: true });
    await useScanStore.getState().refresh();
    expect(commands.scanMachine).not.toHaveBeenCalled();
  });
});
