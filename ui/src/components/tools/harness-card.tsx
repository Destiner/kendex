import { useState } from "react";
import type { Enforcement, HarnessId, ItemKind } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { kindLabel, toolName } from "@/lib/labels";

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

export function HarnessCard({
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
