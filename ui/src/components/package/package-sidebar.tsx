import type {
  ObservedItem,
  PackageFile,
  PackageMeta_Serialize,
  Scope,
  VersionRow,
} from "@/bindings";
import { FileList } from "@/components/package/file-list";
import { PackageMetaBlock } from "@/components/package/package-meta";
import { VersionMenu } from "@/components/package/version-menu";
import { SectionHeading, SettingRow } from "@/components/section";
import { Switch } from "@/components/ui/switch";
import {
  ENABLED_HELP,
  ENABLED_LABEL,
  PACKAGE_FILES_TITLE,
  PACKAGE_VERSION_TITLE,
} from "@/lib/copy";
import type { ItemGroup } from "@/lib/derive";

/** The package page's left column: details, the enabled switch, the
 *  version picker, and the file list. */
export function PackageSidebar({
  group,
  primary,
  meta,
  versions,
  files,
  selectedFile,
  busy,
  onToggle,
  onSwitchVersion,
  onCompare,
  onFollow,
  onSelectFile,
}: {
  group: ItemGroup;
  primary: ObservedItem;
  meta: PackageMeta_Serialize | null;
  versions: VersionRow[];
  files: PackageFile[];
  selectedFile: string | null;
  busy: boolean;
  onToggle: (scope: Scope, enable: boolean) => void;
  onSwitchVersion: (row: VersionRow) => void;
  onCompare: (row: VersionRow) => void;
  onFollow: () => void;
  onSelectFile: (path: string) => void;
}) {
  const managed = group.kind === "agent" || group.kind === "skill";
  const anyDisabled = group.installations.some((i) => i.enabled === false);
  return (
    <div className="w-full shrink-0 space-y-7 lg:w-[24rem]">
      <PackageMetaBlock group={group} primary={primary} meta={meta} />
      {managed ? (
        <SettingRow
          label={ENABLED_LABEL}
          description={ENABLED_HELP}
          htmlFor="package-enabled"
          className="border-y py-3"
        >
          <Switch
            id="package-enabled"
            checked={!anyDisabled}
            disabled={busy}
            onCheckedChange={() => onToggle(primary.scope, anyDisabled)}
          />
        </SettingRow>
      ) : null}
      {versions.length > 0 || meta?.repo ? (
        <div className="space-y-2.5">
          <SectionHeading>{PACKAGE_VERSION_TITLE}</SectionHeading>
          <VersionMenu
            versions={versions}
            held={meta?.rev != null}
            busy={busy}
            onSwitch={onSwitchVersion}
            onCompare={onCompare}
            onFollow={onFollow}
          />
        </div>
      ) : null}
      {files.length > 0 ? (
        <div className="space-y-2.5">
          <SectionHeading>{PACKAGE_FILES_TITLE}</SectionHeading>
          <FileList
            files={files}
            selected={selectedFile}
            onSelect={onSelectFile}
          />
        </div>
      ) : null}
    </div>
  );
}
