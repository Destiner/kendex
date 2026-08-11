import { useEffect } from "react";
import type { EditorInventory, Scope } from "@/bindings";
import { AgentSkillsTab } from "@/components/editor/agent-skills-tab";
import { Field } from "@/components/editor/controls";
import { CustomHooksTab } from "@/components/editor/custom-hooks-tab";
import { FrontmatterTab } from "@/components/editor/frontmatter-tab";
import { InstructionsTab } from "@/components/editor/instructions-tab";
import { SaveBar } from "@/components/editor/save-bar";
import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { type Draft, setProjectSkillsDir } from "@/lib/editor-draft";
import { useEditorStore } from "@/stores/editor";
import { useSettingsStore } from "@/stores/settings";

type Change = (change: (draft: Draft) => Draft) => void;

export function CustomizePage() {
  const {
    scope,
    draft,
    inventory,
    absent,
    dirty,
    loading,
    saving,
    error,
    setScope,
    load,
    edit,
    save,
    create,
  } = useEditorStore();
  const settings = useSettingsStore((s) => s.settings);
  const projects = settings?.projects ?? [];

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="flex min-h-full flex-col">
      <PageHeader
        title="Customize"
        subtitle="Every customization this scope declares — edited here, not by hand"
      />
      <div className="flex-1 space-y-6 p-8">
        <div className="flex items-center gap-3">
          <span className="text-sm text-muted-foreground">Editing</span>
          <Select
            value={scope.scope === "global" ? "global" : scope.root}
            onValueChange={(value) =>
              void setScope(
                value === "global"
                  ? { scope: "global" }
                  : { scope: "project", root: value },
              )
            }
          >
            <SelectTrigger className="w-80" size="sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="global">Global</SelectItem>
              {projects.map((root) => (
                <SelectItem key={root} value={root}>
                  {root}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {error ? (
          <p className="whitespace-pre-wrap text-sm text-destructive">
            {error}
          </p>
        ) : null}
        {loading ? (
          <p className="text-sm text-muted-foreground">loading…</p>
        ) : null}
        {absent ? (
          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              No manifest yet — add something first, or create one.
            </p>
            <Button size="sm" disabled={saving} onClick={() => void create()}>
              Create manifest
            </Button>
          </div>
        ) : null}
        {draft ? (
          <EditorTabs
            draft={draft}
            inventory={inventory}
            scope={scope}
            onChange={edit}
          />
        ) : null}
      </div>
      {dirty ? (
        <SaveBar
          saving={saving}
          onSave={() => void save()}
          onDiscard={() => void load()}
        />
      ) : null}
    </div>
  );
}

function EditorTabs({
  draft,
  inventory,
  scope,
  onChange,
}: {
  draft: Draft;
  inventory: EditorInventory | null;
  scope: Scope;
  onChange: Change;
}) {
  return (
    <Tabs defaultValue="agent-skills">
      <TabsList>
        <TabsTrigger value="agent-skills">Agent skills</TabsTrigger>
        <TabsTrigger value="instructions">Instructions</TabsTrigger>
        <TabsTrigger value="frontmatter">Frontmatter</TabsTrigger>
        <TabsTrigger value="hooks">Custom hooks</TabsTrigger>
        {scope.scope === "project" ? (
          <TabsTrigger value="project">Project skills dir</TabsTrigger>
        ) : null}
      </TabsList>
      <TabsContent value="agent-skills" className="pt-4">
        <AgentSkillsTab
          draft={draft}
          inventory={inventory}
          onChange={onChange}
        />
      </TabsContent>
      <TabsContent value="instructions" className="pt-4">
        <InstructionsTab
          draft={draft}
          inventory={inventory}
          onChange={onChange}
        />
      </TabsContent>
      <TabsContent value="frontmatter" className="pt-4">
        <FrontmatterTab
          draft={draft}
          inventory={inventory}
          onChange={onChange}
        />
      </TabsContent>
      <TabsContent value="hooks" className="pt-4">
        <CustomHooksTab draft={draft} onChange={onChange} />
      </TabsContent>
      <TabsContent value="project" className="max-w-lg pt-4">
        <Field label="project-skills-dir">
          <Input
            aria-label="project-skills-dir"
            placeholder=".claude/skills-src"
            value={draft["project-skills-dir"] ?? ""}
            onChange={(event) =>
              onChange((current) =>
                setProjectSkillsDir(current, event.target.value),
              )
            }
          />
        </Field>
        <p className="pt-2 text-xs text-muted-foreground">
          Project-owned skills live here and are symlinked into the harness
          skill directories, so generated directories stay untracked.
        </p>
      </TabsContent>
    </Tabs>
  );
}
