import { beforeEach, describe, expect, it } from "vitest";
import type { AuditView, ScanResult, SourceRow } from "@/bindings";
import bindingsSource from "../bindings.ts?raw";
import { ACME } from "./fixtures";
import { handlers, mockInvoke, resetMock } from "./mock";

const acme = { scope: "project", root: ACME } as const;

describe("mock bridge", () => {
  beforeEach(resetMock);

  it("has a handler for every generated command, and no extras", () => {
    const names = [
      ...bindingsSource.matchAll(
        /__TAURI_INVOKE(?:<[\s\S]*?>)?\(\s*"([a-z_]+)"/g,
      ),
    ].map((m) => m[1]);
    expect(names.length).toBeGreaterThan(15);
    expect(Object.keys(handlers).sort()).toEqual([...new Set(names)].sort());
  });

  it("rejects unknown commands with a plain string", async () => {
    await expect(mockInvoke("no_such_command")).rejects.toMatch("no handler");
  });

  it("toggle and apply mutate the shared state", async () => {
    await mockInvoke("toggle_item", {
      scope: acme,
      name: "github",
      enabled: false,
    });
    const scan = (await mockInvoke("scan_machine")) as ScanResult;
    const github = scan.items.filter(
      (i) =>
        i.name === "github" &&
        i.scope.scope === "project" &&
        i.scope.root === ACME,
    );
    expect(github.length).toBeGreaterThan(0);
    expect(github.every((i) => i.enabled === false)).toBe(true);

    const after = (await mockInvoke("apply_plan", {
      scope: acme,
      removeOrphans: false,
    })) as AuditView;
    expect(after.plan).toEqual([]);
    expect(after.drift.map((r) => r.state).sort()).toEqual([
      "orphaned",
      "unmanaged",
    ]);
  });

  it("adopting clears the not-managed row and declares the item", async () => {
    const after = (await mockInvoke("adopt_item", {
      scope: acme,
      kind: "skill",
      name: "scratch",
      harness: "claude",
    })) as AuditView;
    expect(after.drift.some((r) => r.name === "scratch")).toBe(false);
    const manifest = (await mockInvoke("get_manifest", { scope: acme })) as {
      skills?: Record<string, { source: string }>;
    };
    expect(manifest.skills?.scratch?.source).toBe("local");
  });

  it("blocks removing a source that still provides items", async () => {
    await expect(
      mockInvoke("source_remove", {
        scope: { scope: "global" },
        name: "vstack",
      }),
    ).rejects.toMatch("disable");
    const rows = (await mockInvoke("source_toggle", {
      scope: { scope: "global" },
      name: "vstack",
      enabled: false,
    })) as SourceRow[];
    const row = rows.find(
      (r) => r.scope.scope === "global" && r.name === "vstack",
    );
    expect(row?.enabled).toBe(false);
  });
});
