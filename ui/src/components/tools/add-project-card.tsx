import { FolderOpen, FolderPlus, FolderSearch } from "lucide-react";
import { useState } from "react";
import { Section } from "@/components/section";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  ADD_PROJECT_HELP,
  NO_PROJECTS_FOUND,
  SCAN_FOLDER_HELP,
} from "@/lib/copy";
import { pickFolder } from "@/lib/pick-folder";

/**
 * A path field with the folder picker inside it, rather than a labelled
 * input, a browse button and a submit button in a row of three — the label
 * only repeated the placeholder, and the picker is part of filling the field
 * in, not a step of its own.
 */
function PathField({
  id,
  placeholder,
  value,
  onChange,
  disabled,
  browseLabel,
}: {
  id: string;
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  browseLabel: string;
}) {
  return (
    <div className="relative max-w-md flex-1">
      <Input
        id={id}
        className="pr-9 font-mono text-[13px]"
        placeholder={placeholder}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
      />
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        className="absolute top-1/2 right-0.5 -translate-y-1/2"
        aria-label={browseLabel}
        title={browseLabel}
        disabled={disabled}
        onClick={() => {
          void pickFolder().then((picked) => {
            if (picked) onChange(picked);
          });
        }}
      >
        <FolderOpen className="size-4" />
      </Button>
    </div>
  );
}

export function AddProjectCard({
  projects,
  registerProject,
  discoverProjects,
}: {
  projects: string[];
  registerProject: (path: string) => Promise<boolean>;
  discoverProjects: (root: string) => Promise<string[]>;
}) {
  const [addPath, setAddPath] = useState("");
  const [adding, setAdding] = useState(false);
  const [discoverRoot, setDiscoverRoot] = useState("");
  const [found, setFound] = useState<string[] | null>(null);

  return (
    <Section title="Add a project" description={ADD_PROJECT_HELP}>
      <form
        className="flex items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          const path = addPath.trim();
          if (!path || adding) return;
          setAdding(true);
          void registerProject(path).then((ok) => {
            setAdding(false);
            if (ok) setAddPath("");
          });
        }}
      >
        <PathField
          id="project-folder"
          placeholder="/path/to/project"
          value={addPath}
          onChange={setAddPath}
          disabled={adding}
          browseLabel="Browse for a project folder"
        />
        <Button type="submit" disabled={adding || !addPath.trim()}>
          <FolderPlus className="size-4" /> Add
        </Button>
      </form>

      <div className="mt-6 flex flex-col gap-2 border-t pt-5">
        <p className="text-[13px] text-muted-foreground">{SCAN_FOLDER_HELP}</p>
        <form
          className="flex items-center gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            if (discoverRoot.trim()) {
              void discoverProjects(discoverRoot.trim()).then(setFound);
            }
          }}
        >
          <PathField
            id="discover-folder"
            placeholder="/path/to/scan"
            value={discoverRoot}
            onChange={setDiscoverRoot}
            browseLabel="Browse for a folder to scan"
          />
          <Button
            type="submit"
            variant="outline"
            disabled={!discoverRoot.trim()}
          >
            <FolderSearch className="size-4" /> Scan
          </Button>
        </form>
        {found ? (
          <div className="flex flex-col pt-1">
            {found.length === 0 ? (
              <p className="text-[13px] text-muted-foreground">
                {NO_PROJECTS_FOUND}
              </p>
            ) : (
              found.map((path) => (
                <div
                  key={path}
                  className="flex items-center justify-between gap-3 border-b border-border/40 py-2 last:border-0"
                >
                  <span className="truncate font-mono text-[13px] text-muted-foreground">
                    {path}
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={projects.includes(path)}
                    onClick={() => void registerProject(path)}
                  >
                    {projects.includes(path) ? "Added" : "Add"}
                  </Button>
                </div>
              ))
            )}
          </div>
        ) : null}
      </div>
    </Section>
  );
}
