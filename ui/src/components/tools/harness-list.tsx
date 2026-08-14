import type { HarnessId } from "@/bindings";
import { HarnessCard } from "@/components/tools/harness-card";
import { Button } from "@/components/ui/button";
import { countByKind } from "@/lib/derive";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

const ALL_HARNESSES: HarnessId[] = [
  "claude",
  "codex",
  "opencode",
  "cursor",
  "pi",
  "gemini",
  "copilot",
];

/** "Tools": the AI coding tools this machine has, one card each. */
export function HarnessList() {
  const result = useScanStore((s) => s.result);
  const refreshScan = useScanStore((s) => s.refresh);
  const { settings, capabilities, setHarnessRoot } = useSettingsStore();

  const anyDetected = ALL_HARNESSES.some((id) =>
    result?.harnesses.some((h) => h.harness === id),
  );

  if (result && !anyDetected) {
    return (
      <div className="flex flex-col items-center gap-2 py-16 text-center">
        <p className="font-medium">No AI coding tools found.</p>
        <p className="text-sm text-muted-foreground">
          Install Claude Code, Codex, OpenCode, Cursor, Pi, Gemini CLI, or
          GitHub Copilot and scan again.
        </p>
        <Button
          variant="outline"
          className="mt-2"
          onClick={() => void refreshScan()}
        >
          Scan again
        </Button>
      </div>
    );
  }

  return (
    <div className="grid gap-4 p-8 lg:grid-cols-2">
      {ALL_HARNESSES.map((id) => {
        const detected = result?.harnesses.find((h) => h.harness === id);
        const items = result?.items.filter((i) => i.harness === id) ?? [];
        const counts = countByKind(items);
        const override = settings?.["harness-roots"]?.[id] ?? "";
        const hooks = capabilities.find(
          (row) => row.harness === id && row.kind === "hook",
        );
        return (
          <HarnessCard
            key={id}
            id={id}
            detectedRoot={detected?.root ?? null}
            version={detected?.version ?? null}
            counts={[...counts.entries()]}
            hooks={hooks?.caps.enforcement ?? "not-applicable"}
            override={override}
            onOverride={(root) => void setHarnessRoot(id, root)}
          />
        );
      })}
    </div>
  );
}
