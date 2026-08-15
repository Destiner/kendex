import type { DriftRow } from "@/bindings";
import { SectionLabel } from "@/components/card-section";
import { Button } from "@/components/ui/button";
import type { MergedDriftRow } from "@/lib/drift-merge";
import { summarizePaths } from "@/lib/drift-merge";
import { kindLabel, toolName, UNMANAGED_SECTION_EXPLAINER } from "@/lib/labels";

// A skill installed by hand for two harnesses is one thing to adopt, not
// two — so one row carries every tool badge and one button adopts every
// installation at once.
export function UnmanagedItems({
  rows,
  busy,
  onAdopt,
}: {
  rows: MergedDriftRow[];
  busy: boolean;
  onAdopt: (
    kind: DriftRow["kind"],
    name: string,
    harness: DriftRow["harness"],
    opts?: { silent?: boolean },
  ) => void;
}) {
  if (rows.length === 0) return null;
  return (
    <div className="space-y-1.5">
      <SectionLabel>
        Not managed yet
        <span className="ml-1.5 font-normal normal-case tracking-normal text-muted-foreground">
          · {rows.length}
        </span>
      </SectionLabel>
      <p className="text-xs text-muted-foreground">
        {UNMANAGED_SECTION_EXPLAINER}
      </p>
      <div className="divide-y divide-border/60 rounded-lg border bg-muted/30">
        {rows.map((group) => {
          const paths = summarizePaths(
            group.installations.map((row) => row.detail),
          );
          const tools = group.installations
            .map((row) => toolName(row.harness))
            .join(", ");
          return (
            <div
              key={`${group.kind}:${group.name}:${group.state}`}
              className="flex items-center gap-2 px-3 py-2.5"
            >
              <span className="min-w-0 flex-1">
                <span className="text-sm font-medium">{group.name}</span>{" "}
                <span className="text-xs text-muted-foreground">
                  {kindLabel(group.kind)} · {tools}
                </span>
                {paths ? (
                  <span
                    className="block truncate font-mono text-xs text-muted-foreground"
                    title={paths.title}
                  >
                    {paths.text}
                  </span>
                ) : null}
              </span>
              <Button
                size="sm"
                variant="ghost"
                className="shrink-0"
                disabled={busy}
                onClick={() => {
                  group.installations.forEach((row, index) => {
                    onAdopt(row.kind, row.name, row.harness, {
                      silent: index > 0,
                    });
                  });
                }}
              >
                Start managing
              </Button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
