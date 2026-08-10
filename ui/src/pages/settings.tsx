import type { Appearance } from "@/bindings";
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
import { useSettingsStore } from "@/stores/settings";

export function SettingsPage() {
  const { settings, error, setAppearance } = useSettingsStore();

  return (
    <div>
      <PageHeader
        title="Settings"
        subtitle="App preferences — one file, nothing hidden"
      />
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
            <CardTitle className="text-base">Registered projects</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-sm text-muted-foreground">
            {(settings?.projects ?? []).length === 0 ? (
              <p>none — register projects on the Scopes page</p>
            ) : (
              settings?.projects?.map((p) => <p key={p}>{p}</p>)
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Harness root overrides</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-sm text-muted-foreground">
            {Object.entries(settings?.["harness-roots"] ?? {}).length === 0 ? (
              <p>none — set overrides on the Harnesses page</p>
            ) : (
              Object.entries(settings?.["harness-roots"] ?? {}).map(
                ([harness, root]) => (
                  <p key={harness}>
                    {harness}: {root}
                  </p>
                ),
              )
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
