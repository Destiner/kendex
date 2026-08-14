import { toast } from "sonner";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "@/bindings";
import { useAuditStore } from "./audit";

vi.mock("@/bindings", () => ({
  commands: {
    auditAll: vi.fn(),
  },
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

describe("audit store refresh", () => {
  beforeEach(() => {
    useAuditStore.setState({
      views: [],
      auditing: false,
      error: null,
      busy: false,
      backgroundFailureAnnounced: false,
    });
    vi.clearAllMocks();
  });

  it("toasts a background audit failure once, not on every silent retry", async () => {
    vi.mocked(commands.auditAll).mockResolvedValue({
      status: "error",
      error: "boom",
    });

    await useAuditStore.getState().refresh();
    await useAuditStore.getState().refresh();

    expect(toast.error).toHaveBeenCalledTimes(1);
  });

  it("re-arms the toast after a successful audit", async () => {
    vi.mocked(commands.auditAll).mockResolvedValueOnce({
      status: "error",
      error: "boom",
    });
    await useAuditStore.getState().refresh();

    vi.mocked(commands.auditAll).mockResolvedValueOnce({
      status: "ok",
      data: [],
    });
    await useAuditStore.getState().refresh();

    vi.mocked(commands.auditAll).mockResolvedValueOnce({
      status: "error",
      error: "boom again",
    });
    await useAuditStore.getState().refresh();

    expect(toast.error).toHaveBeenCalledTimes(2);
  });

  it("does not toast on a successful audit", async () => {
    vi.mocked(commands.auditAll).mockResolvedValue({
      status: "ok",
      data: [],
    });

    await useAuditStore.getState().refresh();

    expect(toast.error).not.toHaveBeenCalled();
  });
});
