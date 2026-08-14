import { useEffect, useState } from "react";
import type { Appearance, HarnessId } from "@/bindings";
import { commands } from "@/bindings";
import { PageHeader } from "@/components/page-header";
import { ToolOverrideRow } from "@/components/tools/tool-override-row";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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

// How cautious the safety check is, said without numbers on the dial.
const SAFETY_LEVELS = {
  strict: { "warn-below": 90, "block-below": 75 },
  balanced: { "warn-below": 80, "block-below": 60 },
  lenient: { "warn-below": 65, "block-below": 40 },
} as const;

type SafetyLevel = keyof typeof SAFETY_LEVELS;

function safetyLevelOf(warn: number, block: number): SafetyLevel | "custom" {
  for (const [name, t] of Object.entries(SAFETY_LEVELS)) {
    if (t["warn-below"] === warn && t["block-below"] === block)
      return name as SafetyLevel;
  }
  return "custom";
}

export function SettingsPage() {
  const { settings, error, setAppearance, setSafety, setHarnessRoot } =
    useSettingsStore();
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    void commands.appVersion().then(setVersion);
  }, []);

  const safety = settings?.safety ?? SAFETY_LEVELS.balanced;
  const level = safetyLevelOf(safety["warn-below"], safety["block-below"]);

  return (
    <div>
      <PageHeader title="Settings" subtitle="Preferences for the app" />
      <div className="p-8">
        <div className="mx-auto w-full max-w-5xl space-y-4">
          {error ? <p className="text-sm text-destructive">{error}</p> : null}

          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium">Appearance</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                <Label>Theme</Label>
                <Select
                  value={settings?.appearance ?? "system"}
                  onValueChange={(value) =>
                    void setAppearance(value as Appearance)
                  }
                >
                  <SelectTrigger className="w-36">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="system">System</SelectItem>
                    <SelectItem value="light">Light</SelectItem>
                    <SelectItem value="dark">Dark</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium">
                Safety check
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                <div className="pr-4">
                  <Label>How cautious</Label>
                  <p className="mt-1 text-xs text-muted-foreground">
                    How readily vstack holds back an item it finds risky before
                    installing it.
                  </p>
                </div>
                <Select
                  value={level === "custom" ? "balanced" : level}
                  onValueChange={(value) => {
                    const t = SAFETY_LEVELS[value as SafetyLevel];
                    void setSafety(t["warn-below"], t["block-below"]);
                  }}
                >
                  <SelectTrigger className="w-36 shrink-0">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="strict">Strict</SelectItem>
                    <SelectItem value="balanced">Balanced</SelectItem>
                    <SelectItem value="lenient">Lenient</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium">Projects</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {(settings?.projects ?? []).length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  No projects yet.
                </p>
              ) : (
                settings?.projects?.map((p) => (
                  <div key={p}>
                    <span className="font-semibold">{p.split("/").pop()}</span>
                    <p className="truncate font-mono text-xs text-muted-foreground">
                      {p}
                    </p>
                  </div>
                ))
              )}
              <p className="text-xs text-muted-foreground">
                Add or remove projects on the Tools & Projects page.
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium">
                Tool folder overrides
              </CardTitle>
            </CardHeader>
            <CardContent className="divide-y">
              {ALL_HARNESSES.map((id) => (
                <ToolOverrideRow
                  key={id}
                  id={id}
                  override={settings?.["harness-roots"]?.[id] ?? ""}
                  onSave={(root) => void setHarnessRoot(id, root)}
                />
              ))}
            </CardContent>
          </Card>

          {version ? (
            <p className="text-xs text-muted-foreground">vstack · {version}</p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
