import { useEffect, useState } from "react";
import type { Appearance, HarnessId } from "@/bindings";
import { commands } from "@/bindings";
import { PageHeader } from "@/components/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { toolName } from "@/lib/labels";
import { useSettingsStore } from "@/stores/settings";

export function SettingsPage() {
  const { settings, error, setAppearance } = useSettingsStore();
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    void commands.appVersion().then(setVersion);
  }, []);

  const overrides = Object.entries(settings?.["harness-roots"] ?? {});

  return (
    <div>
      <PageHeader title="Settings" subtitle="Preferences for the app" />
      <div className="max-w-xl space-y-4 p-8">
        {error ? <p className="text-sm text-destructive">{error}</p> : null}

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Appearance</CardTitle>
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
            <CardTitle className="text-base">Projects</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {(settings?.projects ?? []).length === 0 ? (
              <p className="text-sm text-muted-foreground">No projects yet.</p>
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
              Add or remove projects on the Projects page.
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Tool folder overrides</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-sm">
            {overrides.length === 0 ? (
              <p className="text-muted-foreground">
                None. Set one from the Tools page if a tool lives somewhere
                unusual.
              </p>
            ) : (
              overrides.map(([harness, root]) => (
                <p key={harness} className="text-muted-foreground">
                  <span className="font-medium text-foreground">
                    {toolName(harness as HarnessId)}
                  </span>{" "}
                  {root}
                </p>
              ))
            )}
          </CardContent>
        </Card>

        {version ? (
          <p className="text-center text-xs text-muted-foreground">
            vstack {version}
          </p>
        ) : null}
      </div>
    </div>
  );
}
