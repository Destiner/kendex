// Decision rows for the demo fixtures. The backend issues one per finding,
// aligned by index; the fixture mints them the same way so a demo row
// reads exactly like a real one — every open finding carries a token, and
// a settled one says what settled it.
import type { DecisionState, Finding, FindingDecision } from "@/bindings";

const OPEN: DecisionState = { state: "open", earlier: null };

export function decisionsFor(
  key: string,
  hash: string,
  findings: Finding[],
  states: DecisionState[] = [],
): FindingDecision[] {
  return findings.map((finding, index) => ({
    token: `${key}#${fingerprint(finding, index)}@${hash}`,
    state: states[index] ?? OPEN,
  }));
}

export function accepted(grantedAt: string): DecisionState {
  return { state: "accepted", grantedAt };
}

// A stable stand-in for the backend's fingerprint: sixteen hex characters
// from the rule and where it fired.
function fingerprint(finding: Finding, index: number): string {
  const seed = `${finding.rule}:${finding.location}:${index}`;
  let hash = 0;
  for (const char of seed) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  return hash.toString(16).padStart(8, "0").repeat(2);
}
