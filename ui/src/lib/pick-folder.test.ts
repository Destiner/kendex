import { toast } from "sonner";
import { describe, expect, it, vi } from "vitest";
import { commands } from "@/bindings";
import { pickFolder } from "./pick-folder";

vi.mock("@/bindings", () => ({
  commands: { pickFolder: vi.fn() },
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

describe("pickFolder", () => {
  it("returns the chosen path without a toast", async () => {
    vi.mocked(commands.pickFolder).mockResolvedValue({
      status: "ok",
      data: "/home/x/acme-web",
    });

    expect(await pickFolder()).toBe("/home/x/acme-web");
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("silently returns null on cancel — no toast for a cancelled picker", async () => {
    vi.mocked(commands.pickFolder).mockResolvedValue({
      status: "ok",
      data: null,
    });

    expect(await pickFolder()).toBeNull();
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("toasts and returns null when the picker itself fails", async () => {
    vi.mocked(commands.pickFolder).mockResolvedValue({
      status: "error",
      error: "picker unavailable",
    });

    expect(await pickFolder()).toBeNull();
    expect(toast.error).toHaveBeenCalledWith("picker unavailable");
  });
});
