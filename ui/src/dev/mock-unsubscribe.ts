// Unsubscribing in the mock bridge: the preview partitions what a source's
// going would take (every package it declared, none edited in the mock), and
// the action drops the subscription and its rows.
import type { Scope, UnsubscribePreview } from "@/bindings";
import { type Handler, same, store } from "./mock-state";

const sourceRow = (scope: Scope, source: string) =>
  store.state.sources.find(
    (row) => same(row.scope, scope) && row.name === source,
  );

export const unsubscribeHandlers: Record<string, Handler> = {
  marketplace_unsubscribe_preview: ({
    scope,
    source,
  }: {
    scope: Scope;
    source: string;
  }): UnsubscribePreview => {
    const row = sourceRow(scope, source);
    return {
      // The mock's declared items are bare names; the dialog only needs
      // something to list, so each reads as a skill.
      removable: (row?.declaredItems ?? []).map((name) => ({
        kind: "skill" as const,
        name,
      })),
      edited: [],
      bundles: [],
    };
  },

  marketplace_unsubscribe: ({
    scope,
    source,
  }: {
    scope: Scope;
    source: string;
    keep: boolean;
    discardEdits: boolean;
  }): null => {
    store.state.marketplaces = store.state.marketplaces.filter(
      (row) => !(same(row.scope, scope) && row.name === source),
    );
    store.state.sources = store.state.sources.filter(
      (row) => !(same(row.scope, scope) && row.name === source),
    );
    return null;
  },
};
