import { SlidersHorizontal } from "lucide-react";
import { useEffect } from "react";
import type { EditorInventory, Scope } from "@/bindings";
import { AgentSkillsTab } from "@/components/editor/agent-skills-tab";
import { Field } from "@/components/editor/controls";
import { CustomHooksTab } from "@/components/editor/custom-hooks-tab";
import { FrontmatterTab } from "@/components/editor/frontmatter-tab";
import { InstructionsTab } from "@/components/editor/instructions-tab";
import { SaveBar } from "@/components/editor/save-bar";
import { EmptyState } from "@/components/empty-state";
import { PageHeader } from "@/components/page-header";
import { StatusNote } from "@/components/status-note";
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
import {
  CUSTOMIZE_SUBTITLE,
  NOTHING_CUSTOMIZED,
  NOTHING_CUSTOMIZED_BODY,
  START_CUSTOMIZING,
} from "@/lib/copy-customize";
import { type Draft, setProjectSkillsDir } from "@/lib/editor-draft";
import { CONTENT_WIDTH, PAGE_BODY } from "@/lib/layout";
import { cn } from "@/lib/utils";
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
        subtitle={CUSTOMIZE_SUBTITLE}
        action={
          <div className="flex items-center gap-2">
            <span className="text-[13px] text-muted-foreground">Editing</span>
            <Select
              value={scope.scope === "global" ? "global" : scope.root}
              onValueChange={(value) => {
                if (value === null) return;
                void setScope(
                  value === "global"
                    ? { scope: "global" }
                    : { scope: "project", root: value },
                );
              }}
            >
              <SelectTrigger className="w-56" size="sm">
                <SelectValue>
                  {(value: string) =>
                    value === "global" ? "Everything (global)" : value
                  }
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="global">Everything (global)</SelectItem>
                {projects.map((root) => (
                  <SelectItem key={root} value={root}>
                    {root}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        }
      />
      <div className={cn("flex-1", PAGE_BODY)}>
        <div className={cn("space-y-6", CONTENT_WIDTH)}>
          {error ? (
            <StatusNote tone="critical" title="That change couldn't be saved">
              <span className="whitespace-pre-wrap">{error}</span>
            </StatusNote>
          ) : null}
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : null}
          {absent ? (
            <EmptyState
              icon={SlidersHorizontal}
              title={NOTHING_CUSTOMIZED}
              action={
                <Button disabled={saving} onClick={() => void create()}>
                  {START_CUSTOMIZING}
                </Button>
              }
            >
              {NOTHING_CUSTOMIZED_BODY}
            </EmptyState>
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
        <TabsTrigger value="frontmatter">Agent settings</TabsTrigger>
        <TabsTrigger value="hooks">Custom hooks</TabsTrigger>
        {scope.scope === "project" ? (
          <TabsTrigger value="project">Project skills folder</TabsTrigger>
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
        <Field label="Skills folder">
          <Input
            aria-label="Skills folder"
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
          Skills you own live here, then get linked into each tool's own skill
          folder, so the generated copies don't need to be committed.
        </p>
      </TabsContent>
    </Tabs>
  );
}
