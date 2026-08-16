import { ChevronLeft } from "lucide-react";
import type { PackageDiff } from "@/bindings";
import { DiffFile } from "@/components/diff/diff-file";
import { SectionHeading } from "@/components/section";
import { Button } from "@/components/ui/button";
import { BACK_TO_FILES_LABEL, DIFF_TRUNCATED_NOTE } from "@/lib/copy";
import {
  additionsLabel,
  deletionsLabel,
  lineNumberWidth,
  openByDefault,
} from "@/lib/diff";

/** The changes between two versions: a totals header and one disclosure
 *  per changed file. Serves version comparison, update previews, and the
 *  fork "what did I change" view — same shape everywhere. */
export function DiffView({
  diff,
  fromLabel,
  toLabel,
  onClose,
}: {
  diff: PackageDiff;
  fromLabel: string;
  toLabel: string;
  onClose?: () => void;
}) {
  const gutterCh = lineNumberWidth(diff);
  const expanded = openByDefault(diff);
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        {onClose ? (
          <Button variant="ghost" size="sm" onClick={onClose}>
            <ChevronLeft className="size-3.5" />
            {BACK_TO_FILES_LABEL}
          </Button>
        ) : null}
        <SectionHeading>Changes</SectionHeading>
        <span className="text-sm text-muted-foreground">
          {fromLabel} → {toLabel}
        </span>
        <span className="ml-auto flex gap-2 font-mono text-sm tabular-nums">
          <span className="text-good">
            {additionsLabel(diff.totalAdditions)}
          </span>
          <span className="text-critical">
            {deletionsLabel(diff.totalDeletions)}
          </span>
        </span>
      </div>
      {diff.truncated ? (
        <p className="text-xs text-muted-foreground">{DIFF_TRUNCATED_NOTE}</p>
      ) : null}
      {diff.files.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          These versions have identical files.
        </p>
      ) : (
        <div className="space-y-2">
          {diff.files.map((file) => (
            <DiffFile
              key={file.path}
              file={file}
              gutterCh={gutterCh}
              defaultOpen={expanded}
            />
          ))}
        </div>
      )}
    </div>
  );
}
