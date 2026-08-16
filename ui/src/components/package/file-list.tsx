import { FileText } from "lucide-react";
import type { PackageFile } from "@/bindings";
import { README_TAG } from "@/lib/copy";
import { cn } from "@/lib/utils";

/** Bytes as a person reads them at a glance. */
export function fileSizeLabel(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${Math.round(size / 1024)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

/** The package's files as a flat, directory-indented list. Package trees
 *  are small; a collapsible tree would be more chrome than content. */
export function FileList({
  files,
  selected,
  onSelect,
}: {
  files: PackageFile[];
  selected: string | null;
  onSelect: (path: string) => void;
}) {
  return (
    <ul className="space-y-0.5">
      {files.map((file) => {
        const depth = file.path.split("/").length - 1;
        const basename = file.path.split("/").pop() ?? file.path;
        return (
          <li key={file.path}>
            <button
              type="button"
              className={cn(
                "flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-accent",
                selected === file.path && "bg-muted/60",
              )}
              style={{ paddingLeft: `${0.5 + depth * 1}rem` }}
              onClick={() => onSelect(file.path)}
              title={file.path}
            >
              <FileText className="size-3.5 shrink-0 text-muted-foreground" />
              <span className="min-w-0 truncate">{basename}</span>
              {file.isReadme ? (
                <span className="shrink-0 text-xs text-muted-foreground">
                  {README_TAG}
                </span>
              ) : null}
              <span className="ml-auto shrink-0 text-xs text-muted-foreground tabular-nums">
                {fileSizeLabel(file.size)}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
