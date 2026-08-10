import { useState } from "react";
import { PageHeader } from "@/components/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { countByKind } from "@/lib/derive";
import { useScanStore } from "@/stores/scan";
import { useSettingsStore } from "@/stores/settings";

const ALL_HARNESSES = ["claude", "codex", "opencode", "cursor", "pi"];

export function HarnessesPage() {
  const result = useScanStore((s) => s.result);
  const { settings, setHarnessRoot } = useSettingsStore();

  return (
    <div>
      <PageHeader title="Harnesses" subtitle="Detected tools and their roots" />
      <div className="grid gap-4 p-8 lg:grid-cols-2">
        {ALL_HARNESSES.map((id) => {
          const detected = result?.harnesses.find((h) => h.harness === id);
          const items = result?.items.filter((i) => i.harness === id) ?? [];
          const counts = countByKind(items);
          const override = settings?.["harness-roots"]?.[id] ?? "";
          return (
            <HarnessCard
              key={id}
              id={id}
              detectedRoot={detected?.root ?? null}
              counts={[...counts.entries()]}
              override={override}
              onOverride={(root) => void setHarnessRoot(id, root)}
            />
          );
        })}
      </div>
    </div>
  );
}

function HarnessCard({
  id,
  detectedRoot,
  counts,
  override,
  onOverride,
}: {
  id: string;
  detectedRoot: string | null;
  counts: [string, number][];
  override: string;
  onOverride: (root: string) => void;
}) {
  const [draft, setDraft] = useState(override);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          {id}
          {detectedRoot ? (
            <Badge variant="secondary">detected</Badge>
          ) : (
            <Badge variant="outline">not detected</Badge>
          )}
        </CardTitle>
        {detectedRoot ? (
          <p className="break-all text-xs text-muted-foreground">
            {detectedRoot}
          </p>
        ) : null}
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex flex-wrap gap-1.5">
          {counts.length === 0 ? (
            <span className="text-sm text-muted-foreground">
              nothing observed
            </span>
          ) : (
            counts.map(([kind, count]) => (
              <Badge key={kind} variant="outline">
                {kind} {count}
              </Badge>
            ))
          )}
        </div>
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            onOverride(draft);
          }}
        >
          <Input
            placeholder="Global root override"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
          />
          <Button type="submit" variant="outline" size="sm" className="h-9">
            Set
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
