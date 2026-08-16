// Grouping logic for the "Held back" panel specifically — split out of
// group-findings.ts to stay under the file's line cap. A held-back skill
// installed on several harnesses that share the same files on disk, or a
// single rule firing at several locations in one skill, otherwise renders
// as several verbatim repeats of the same problem; this collapses both
// before anything renders.
import type { Finding, ItemKind, ItemSafety, Severity } from "@/bindings";
import { SEVERITY_RANK } from "@/lib/group-findings";

export interface RuleGroup {
  rule: string;
  severity: Severity;
  message: string;
  remediation: string;
  locations: string[];
}

// Within one item's finding list, the same rule can fire once per line it
// matched (a hook shelling through the same wrapper at four call sites) —
// same message, same fix, four locations. This collapses those into one
// entry so the fix sentence prints once instead of once per location.
export function groupFindingsByRule(findings: Finding[]): RuleGroup[] {
  const ordered: RuleGroup[] = [];
  const byKey = new Map<string, RuleGroup>();
  for (const finding of findings) {
    const key = `${finding.rule}::${finding.message}::${finding.remediation}`;
    let group = byKey.get(key);
    if (!group) {
      group = {
        rule: finding.rule,
        severity: finding.severity,
        message: finding.message,
        remediation: finding.remediation,
        locations: [],
      };
      byKey.set(key, group);
      ordered.push(group);
    }
    if (SEVERITY_RANK[finding.severity] > SEVERITY_RANK[group.severity]) {
      group.severity = finding.severity;
    }
    group.locations.push(finding.location);
  }
  return ordered;
}

// Every location in a rule-group's list tends to share a long directory
// prefix (the skill's own folder) — printing it on each line just makes the
// list harder to scan. This strips the longest shared prefix once, trimmed

export interface BlockedGroup {
  kind: ItemKind;
  name: string;
  /** One row per harness this exact finding set was seen on. */
  rows: ItemSafety[];
  findingGroups: RuleGroup[];
}

// The engine emits one blocked ItemSafety per harness a skill is installed
// on. When two harnesses share the same files on disk, they carry the exact
// same rule hitting the exact same locations — rendered separately that's
// two verbatim panels for one logical problem. This merges rows sharing
// (kind, name) whose finding sets are identical (same rule, message,
// remediation, and location, as a multiset) into one entry carrying every
// harness it was seen on. A name that means something different per
// harness — same skill name, different files, different findings — stays
// separate, since collapsing it would hide a real difference.
export function groupBlocked(blocked: ItemSafety[]): BlockedGroup[] {
  const ordered: BlockedGroup[] = [];
  const byKey = new Map<string, BlockedGroup>();
  for (const row of blocked) {
    const setKey = row.findings
      .map((f) => `${f.rule}::${f.message}::${f.remediation}::${f.location}`)
      .sort()
      .join("|");
    const key = `${row.kind}::${row.name}::${setKey}`;
    let group = byKey.get(key);
    if (!group) {
      group = {
        kind: row.kind,
        name: row.name,
        rows: [],
        findingGroups: groupFindingsByRule(row.findings),
      };
      byKey.set(key, group);
      ordered.push(group);
    }
    group.rows.push(row);
  }
  return ordered;
}
