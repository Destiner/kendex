import { useCallback, useEffect, useState } from "react";
import {
  commands,
  type HarnessId,
  type PackageDiff,
  type PackageFile,
  type PackageMeta_Serialize,
  type VersionRow,
} from "@/bindings";
import type { PackageRef } from "@/stores/nav";
import { useProblemsStore } from "@/stores/problems";

export type PackageView =
  | { mode: "files"; file: string | null }
  | {
      mode: "diff";
      from: string;
      to: string;
      fromLabel: string;
      toLabel: string;
    };

/** The package page's reads, refetchable as one unit after a mutation. */
export function usePackageData(ref: PackageRef | null) {
  const [meta, setMeta] = useState<PackageMeta_Serialize | null>(null);
  const [files, setFiles] = useState<PackageFile[]>([]);
  const [versions, setVersions] = useState<VersionRow[]>([]);

  const load = useCallback(() => {
    if (!ref) return;
    void commands
      .packageMeta(ref.scope, ref.kind, ref.name)
      .then((response) => {
        setMeta(response.status === "ok" ? response.data : null);
      });
    void commands
      .packageFiles(ref.scope, ref.kind, ref.name)
      .then((response) => {
        setFiles(response.status === "ok" ? response.data : []);
      });
    void commands
      .packageVersions(ref.scope, ref.kind, ref.name)
      .then((response) => {
        setVersions(response.status === "ok" ? response.data : []);
      });
  }, [ref]);

  useEffect(load, [load]);
  return { meta, files, versions, load };
}

/** The diff behind a diff view, fetched when the view asks for one. The
 *  special id "installed" compares against what is on disk. */
export function usePackageDiff(
  ref: PackageRef | null,
  view: PackageView,
  harness: HarnessId | null,
) {
  const showError = useProblemsStore((s) => s.showError);
  const [diff, setDiff] = useState<PackageDiff | null>(null);

  useEffect(() => {
    if (!ref || view.mode !== "diff") {
      setDiff(null);
      return;
    }
    let cancelled = false;
    setDiff(null);
    const sel = (id: string) =>
      id === "installed"
        ? ({ at: "installed" } as const)
        : ({ at: "commit", commit: id } as const);
    void commands
      .packageDiff(
        ref.scope,
        ref.kind,
        ref.name,
        sel(view.from),
        sel(view.to),
        harness,
      )
      .then((response) => {
        if (cancelled) return;
        if (response.status === "ok") setDiff(response.data);
        else showError({ title: "Couldn't compare", message: response.error });
      });
    return () => {
      cancelled = true;
    };
  }, [ref, view, harness, showError]);

  return diff;
}
