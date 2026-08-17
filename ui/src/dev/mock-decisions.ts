// The decision commands over the in-memory fixture: a dismissal flips the
// finding's decision on the safety row it belongs to, and the registry is
// read back off those rows — so the Settings list and the Review page can
// never disagree in the demo either.
import type {
  DecisionState,
  DismissReason,
  ItemSafety,
  RecordedDecision,
  Scope,
} from "@/bindings";
import { type Handler, same, store, view } from "./mock-state";

const key = (row: ItemSafety) => `${row.kind}:${row.name}:${row.harness}`;

function rows(): ItemSafety[] {
  return store.state.views.flatMap((v) => v.safety);
}

function listDecisions(): RecordedDecision[] {
  const out: RecordedDecision[] = [];
  for (const row of rows()) {
    if (row.override.state === "active") {
      out.push({
        scope: row.scope,
        key: key(row),
        kind: row.kind,
        name: row.name,
        harness: row.harness,
        record: {
          kind: "accepted",
          findings: row.findings.length,
          grantedAt: "2026-08-10T09:12:00Z",
        },
        state: { state: "active" },
      });
    }
    row.decisions.forEach((decision, index) => {
      if (decision.state.state !== "dismissed" || !decision.token) return;
      out.push({
        scope: row.scope,
        key: key(row),
        kind: row.kind,
        name: row.name,
        harness: row.harness,
        record: {
          kind: "dismissed",
          fingerprint: decision.fingerprint,
          reason: decision.state.reason,
          dismissedAt: decision.state.dismissedAt,
          finding: row.findings[index] ?? null,
        },
        state: { state: "active" },
      });
    });
  }
  return out;
}

function setDecision(
  scope: Scope,
  match: (token: string) => boolean,
  state: DecisionState,
) {
  for (const v of store.state.views) {
    if (!same(v.scope, scope)) continue;
    for (const row of v.safety) {
      for (const decision of row.decisions) {
        if (decision.token && match(decision.token)) decision.state = state;
      }
    }
  }
}

export const decisionHandlers: Record<string, Handler> = {
  list_decisions: () => ({ decisions: listDecisions(), errors: [] }),
  dismiss_findings: ({
    scope,
    tokens,
    reason,
  }: {
    scope: Scope;
    tokens: string[];
    reason: DismissReason;
  }) => {
    const dismissedAt = new Date().toISOString().replace(/\.\d+Z$/, "Z");
    setDecision(scope, (token) => tokens.includes(token), {
      state: "dismissed",
      reason,
      dismissedAt,
    });
    return { view: view(scope), dismissedAt };
  },
  revoke_dismissal: ({
    scope,
    key: itemKey,
    fingerprint,
  }: {
    scope: Scope;
    key: string;
    fingerprint: string;
  }) => {
    setDecision(
      scope,
      (token) => token.startsWith(`${itemKey}#${fingerprint}@`),
      { state: "open", earlier: null },
    );
    return view(scope);
  },
  revoke_safety_override: ({
    scope,
    key: itemKey,
  }: {
    scope: Scope;
    key: string;
  }) => {
    for (const v of store.state.views) {
      if (!same(v.scope, scope)) continue;
      for (const row of v.safety) {
        if (key(row) === itemKey) row.override = { state: "absent" };
      }
    }
    return view(scope);
  },
};
