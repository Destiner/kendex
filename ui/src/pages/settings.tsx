import { useEffect, useState } from "react";
import type { Appearance, HarnessId } from "@/bindings";
import { commands } from "@/bindings";
import { AcceptedOverrides } from "@/components/accepted-overrides";
import { PageHeader } from "@/components/page-header";
import { Section, SettingRow } from "@/components/section";
import { ToolOverrideRow } from "@/components/tools/tool-override-row";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SAFETY_HELP } from "@/lib/copy-safety";
import { SETTINGS_SUBTITLE } from "@/lib/labels";
import { CONTENT_WIDTH, PAGE_BODY } from "@/lib/layout";
import { cn } from "@/lib/utils";
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

const THEME_LABELS: Record<Appearance, string> = {
  system: "System",
  light: "Light",
  dark: "Dark",
};

const SAFETY_LABELS: Record<SafetyLevel, string> = {
  strict: "Strict",
  balanced: "Balanced",
  lenient: "Lenient",
};

function safetyLevelOf(warn: number, block: number): SafetyLevel | "custom" {
  for (const [name, t] of Object.entries(SAFETY_LEVELS)) {
    if (t["warn-below"] === warn && t["block-below"] === block)
      return name as SafetyLevel;
  }
  return "custom";
}

export function SettingsPage() {
  const { settings, setAppearance, setSafety, setHarnessRoot } =
    useSettingsStore();
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    void commands.appVersion().then(setVersion);
  }, []);

  const safety = settings?.safety ?? SAFETY_LEVELS.balanced;
  const level = safetyLevelOf(safety["warn-below"], safety["block-below"]);
  const projects = settings?.projects ?? [];

  return (
    <div>
      <PageHeader title="Settings" subtitle={SETTINGS_SUBTITLE} />
      <div className={PAGE_BODY}>
        <div className={cn("flex flex-col gap-10", CONTENT_WIDTH)}>
          <Section title="Appearance">
            <SettingRow
              label="Theme"
              description="Follows your system by default."
            >
              <Select
                value={settings?.appearance ?? "system"}
                onValueChange={(value) =>
                  void setAppearance(value as Appearance)
                }
              >
                <SelectTrigger className="w-40">
                  <SelectValue>
                    {(value: Appearance) => THEME_LABELS[value]}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="system">System</SelectItem>
                  <SelectItem value="light">Light</SelectItem>
                  <SelectItem value="dark">Dark</SelectItem>
                </SelectContent>
              </Select>
            </SettingRow>
          </Section>

          <Section title="Safety check">
            <SettingRow label="How cautious" description={SAFETY_HELP}>
              <Select
                value={level === "custom" ? "balanced" : level}
                onValueChange={(value) => {
                  const t = SAFETY_LEVELS[value as SafetyLevel];
                  void setSafety(t["warn-below"], t["block-below"]);
                }}
              >
                <SelectTrigger className="w-40">
                  <SelectValue>
                    {(value: SafetyLevel) => SAFETY_LABELS[value]}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="strict">Strict</SelectItem>
                  <SelectItem value="balanced">Balanced</SelectItem>
                  <SelectItem value="lenient">Lenient</SelectItem>
                </SelectContent>
              </Select>
            </SettingRow>
          </Section>

          <AcceptedOverrides />

          <Section
            title="Projects"
            description="Add or remove projects on the Tools & Projects page."
          >
            {projects.length === 0 ? (
              <p className="py-3.5 text-sm text-muted-foreground">
                No projects yet.
              </p>
            ) : (
              projects.map((path) => (
                <SettingRow
                  key={path}
                  label={path.split("/").pop()}
                  description={<span className="font-mono">{path}</span>}
                />
              ))
            )}
          </Section>

          <Section
            title="Tool folders"
            description="Where each tool keeps its files. Set one only if you've moved a tool somewhere other than its usual place."
          >
            {ALL_HARNESSES.map((id) => (
              <ToolOverrideRow
                key={id}
                id={id}
                override={settings?.["harness-roots"]?.[id] ?? ""}
                onSave={(root) => void setHarnessRoot(id, root)}
              />
            ))}
          </Section>

          <Section title="About">
            <SettingRow
              label={
                <span className="flex items-baseline gap-2">
                  Version
                  <span className="font-mono text-xs font-normal text-muted-foreground">
                    {version ?? "…"}
                  </span>
                </span>
              }
              description="vstack keeps your AI coding tools in sync."
            />
          </Section>
        </div>
      </div>
    </div>
  );
}
