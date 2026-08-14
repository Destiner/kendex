import { toast } from "sonner";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppSettings } from "@/bindings";
import { commands } from "@/bindings";
import { useScanStore } from "./scan";
import { useSettingsStore } from "./settings";

vi.mock("@/bindings", () => ({
  commands: {
    getSettings: vi.fn(),
    capabilityTable: vi.fn(),
    updateSettings: vi.fn(),
    registerProject: vi.fn(),
    unregisterProject: vi.fn(),
    discoverProjects: vi.fn(),
    scanMachine: vi.fn(),
  },
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

const settings: AppSettings = {
  schema: 1,
  appearance: "system",
  safety: { "warn-below": 80, "block-below": 60 },
  "harness-roots": {},
  projects: [],
};

describe("settings store", () => {
  beforeEach(() => {
    useSettingsStore.setState({ settings: null, capabilities: [] });
    useScanStore.setState({
      result: null,
      scanning: false,
      error: null,
      lastScanAt: null,
      backgroundFailureAnnounced: false,
    });
    vi.clearAllMocks();
    vi.mocked(commands.scanMachine).mockResolvedValue({
      status: "ok",
      data: { harnesses: [], items: [], missingProjects: [], warnings: [] },
    });
  });

  it("toasts a plain-words message and leaves settings untouched when a save fails", async () => {
    useSettingsStore.setState({ settings });
    vi.mocked(commands.updateSettings).mockResolvedValue({
      status: "error",
      error: "disk is full",
    });

    await useSettingsStore.getState().setAppearance("dark");

    expect(toast.error).toHaveBeenCalledWith("disk is full");
    expect(useSettingsStore.getState().settings).toBe(settings);
  });

  it("saves settings silently on success — no toast for an instant, visible change", async () => {
    const updated = { ...settings, appearance: "dark" as const };
    useSettingsStore.setState({ settings });
    vi.mocked(commands.updateSettings).mockResolvedValue({
      status: "ok",
      data: updated,
    });

    await useSettingsStore.getState().setAppearance("dark");

    expect(toast.success).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
    expect(useSettingsStore.getState().settings).toEqual(updated);
  });

  it("toasts success naming the folder when a project is added, and resolves true", async () => {
    vi.mocked(commands.registerProject).mockResolvedValue({
      status: "ok",
      data: { ...settings, projects: ["/home/x/acme-web"] },
    });

    const ok = await useSettingsStore
      .getState()
      .registerProject("/home/x/acme-web");

    expect(ok).toBe(true);
    expect(toast.success).toHaveBeenCalledWith("Added acme-web");
  });

  it("toasts failure and resolves false when adding a project fails, without touching settings", async () => {
    useSettingsStore.setState({ settings });
    vi.mocked(commands.registerProject).mockResolvedValue({
      status: "error",
      error: "project already registered: /home/x/acme-web",
    });

    const ok = await useSettingsStore
      .getState()
      .registerProject("/home/x/acme-web");

    expect(ok).toBe(false);
    expect(toast.success).not.toHaveBeenCalled();
    expect(toast.error).toHaveBeenCalledWith(
      "project already registered: /home/x/acme-web",
    );
    expect(useSettingsStore.getState().settings).toBe(settings);
  });

  it("toasts failure without a success toast when removing a project fails", async () => {
    useSettingsStore.setState({ settings });
    vi.mocked(commands.unregisterProject).mockResolvedValue({
      status: "error",
      error: "project not registered: /home/x/gone",
    });

    await useSettingsStore.getState().unregisterProject("/home/x/gone");

    expect(toast.error).toHaveBeenCalledWith(
      "project not registered: /home/x/gone",
    );
    expect(toast.success).not.toHaveBeenCalled();
  });

  it("toasts on a failed load instead of storing a buried error", async () => {
    vi.mocked(commands.getSettings).mockResolvedValue({
      status: "error",
      error: "cannot locate the home directory on this system",
    });
    vi.mocked(commands.capabilityTable).mockResolvedValue([]);

    await useSettingsStore.getState().load();

    expect(toast.error).toHaveBeenCalledWith(
      "cannot locate the home directory on this system",
    );
    expect(useSettingsStore.getState().settings).toBeNull();
  });

  it("toasts and returns an empty list when discovering projects fails", async () => {
    vi.mocked(commands.discoverProjects).mockResolvedValue({
      status: "error",
      error: "/nope is not a directory",
    });

    const found = await useSettingsStore.getState().discoverProjects("/nope");

    expect(found).toEqual([]);
    expect(toast.error).toHaveBeenCalledWith("/nope is not a directory");
  });
});
