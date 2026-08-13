import { useState } from "react";
import type { Enforcement, HarnessId, ItemKind } from "@/bindings";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { countByKind } from "@/lib/derive";
import { kindLabel, toolName } from "@/lib/labels";
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

// What a safety hook installed here actually does. A tool that only reads
// the file must never look like one that acts on it.
function hookLine(name: string, enforcement: Enforcement): string {
  switch (enforcement) {
    case "enforced":
      return `${name} runs safety hooks and honors what they answer.`;
    case "advisory":
      return `${name} has no hooks of its own: a safety hook installs here as text the model can ignore.`;
    case "not-applicable":
      return `${name} has no place for safety hooks.`;
  }
}

export function HarnessesPage() {
  const result = useScanStore((s) => s.result);
  const refreshScan = useScanStore((s) => s.refresh);
  const { settings, capabilities, setHarnessRoot } = useSettingsStore();

  const anyDetected = ALL_HARNESSES.some((id) =>
    result?.harnesses.some((h) => h.harness === id),
  );

  return (
    <div>
      <PageHeader
        title="Tools"
        subtitle="The AI coding tools on this machine"
      />
      {result && !anyDetected ? (
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
      ) : (
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
      )}
    </div>
  );
}

function HarnessCard({
  id,
  detectedRoot,
  version,
  counts,
  hooks,
  override,
  onOverride,
}: {
  id: HarnessId;
  detectedRoot: string | null;
  version: string | null;
  counts: [ItemKind, number][];
  hooks: Enforcement;
  override: string;
  onOverride: (root: string) => void;
}) {
  const [draft, setDraft] = useState(override);
  const name = toolName(id);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          {name}
          {detectedRoot ? (
            <Badge variant="secondary">Detected</Badge>
          ) : (
            <Badge variant="outline">Not installed</Badge>
          )}
        </CardTitle>
        {version ? (
          <p className="text-xs text-muted-foreground">{version}</p>
        ) : null}
        {detectedRoot ? (
          <p className="break-all font-mono text-xs text-muted-foreground">
            {detectedRoot}
          </p>
        ) : null}
      </CardHeader>
      <CardContent className="space-y-3">
        <p className="text-xs text-muted-foreground">{hookLine(name, hooks)}</p>
        <div className="flex flex-wrap gap-1.5">
          {counts.length === 0 ? (
            <span className="text-sm text-muted-foreground">
              Nothing from vstack yet.
            </span>
          ) : (
            counts.map(([kind, count]) => (
              <Badge key={kind} variant="outline">
                {count} {kindLabel(kind, count)}
              </Badge>
            ))
          )}
        </div>
        <form
          className="space-y-1.5"
          onSubmit={(e) => {
            e.preventDefault();
            onOverride(draft);
          }}
        >
          <Label htmlFor={`${id}-root`}>Folder override</Label>
          <p className="text-xs text-muted-foreground">
            Only set this if {name} keeps its files somewhere unusual.
          </p>
          <div className="flex gap-2">
            <Input
              id={`${id}-root`}
              placeholder="/path/to/folder"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
            />
            <Button type="submit" variant="outline">
              Save
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
