import { useEffect } from "react";
import { CustomHooks } from "@/components/customize/custom-hooks";
import { CustomizedIndex } from "@/components/customize/customized-index";
import { SaveBar } from "@/components/customize/save-bar";
import { SharedInstructions } from "@/components/customize/shared-instructions";
import { PageHeader } from "@/components/page-header";
import { Section } from "@/components/section";
import { StatusNote } from "@/components/status-note";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  CUSTOMIZE_SUBTITLE,
  CUSTOMIZED_SECTION,
  CUSTOMIZED_SECTION_HELP,
  HOOKS_HELP,
  HOOKS_SECTION,
  SHARED_SECTION,
  SHARED_SECTION_HELP,
  SKILLS_DIR_HELP,
  SKILLS_DIR_SECTION,
} from "@/lib/copy-customize";
import {
  clearItemCustomization,
  customizedItems,
  sharedCustomization,
} from "@/lib/customization";
import { setProjectSkillsDir } from "@/lib/editor-draft";
import { CONTENT_WIDTH, PAGE_BODY } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { useEditorStore } from "@/stores/editor";
import { useSettingsStore } from "@/stores/settings";

/** What you have changed that isn't about one package — instructions every
 *  agent and skill gets, hooks of your own, where a project keeps its
 *  skills — and the way in to everything that is. */
export function CustomizePage() {
  const {
    scope,
    draft,
    dirty,
    loading,
    saving,
    error,
    setScope,
    load,
    edit,
    save,
  } = useEditorStore();
  const projects = useSettingsStore((s) => s.settings?.projects ?? []);

  // Unsaved edits made on a package's own page live in this same draft;
  // reloading over them here would throw away work with nothing said.
  useEffect(() => {
    if (!useEditorStore.getState().dirty) void load();
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
        <div className={cn("flex flex-col gap-10", CONTENT_WIDTH)}>
          {error ? (
            <StatusNote tone="critical" title="That change couldn't be saved">
              <span className="whitespace-pre-wrap">{error}</span>
            </StatusNote>
          ) : null}
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : null}
          {draft ? (
            <>
              <Section title={SHARED_SECTION} description={SHARED_SECTION_HELP}>
                <SharedInstructions
                  shared={sharedCustomization(draft)}
                  onChange={edit}
                />
              </Section>
              <Section
                title={CUSTOMIZED_SECTION}
                description={CUSTOMIZED_SECTION_HELP}
              >
                <CustomizedIndex
                  items={customizedItems(draft)}
                  scope={scope}
                  onRemove={(kind, name) =>
                    edit((current) =>
                      clearItemCustomization(current, kind, name),
                    )
                  }
                />
              </Section>
              <Section title={HOOKS_SECTION} description={HOOKS_HELP}>
                <CustomHooks draft={draft} onChange={edit} />
              </Section>
              {scope.scope === "project" ? (
                <Section
                  title={SKILLS_DIR_SECTION}
                  description={SKILLS_DIR_HELP}
                >
                  <Input
                    aria-label="Skills folder"
                    placeholder=".claude/skills-src"
                    className="max-w-lg"
                    value={draft["project-skills-dir"] ?? ""}
                    onChange={(event) =>
                      edit((current) =>
                        setProjectSkillsDir(current, event.target.value),
                      )
                    }
                  />
                </Section>
              ) : null}
            </>
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
