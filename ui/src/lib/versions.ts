import type { VersionRef, VersionRow } from "@/bindings";

/** A version as a person reads it: the release name when it has one, a
 *  short commit id otherwise. */
export function versionLabel(
  version: Pick<VersionRef, "commit" | "label">,
): string {
  return version.label ?? version.commit.slice(0, 7);
}

export function versionRowLabel(row: VersionRow): string {
  return row.label ?? row.id.slice(0, 7);
}

/** The row a fresh "Update" should land on: the newest one. */
export function latestRow(rows: VersionRow[]): VersionRow | undefined {
  return rows[0];
}

export function installedRow(rows: VersionRow[]): VersionRow | undefined {
  return rows.find((row) => row.installed);
}
