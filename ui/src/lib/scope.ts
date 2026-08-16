import type { Scope } from "@/bindings";

/** A scope as a stable key: "global", or the project root path. */
export const scopeKey = (scope: Scope): string =>
  scope.scope === "global" ? "global" : scope.root;

/** Whether two scopes are the same place. */
export const sameScope = (a: Scope, b: Scope): boolean =>
  scopeKey(a) === scopeKey(b);
