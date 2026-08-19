import { create } from "zustand";
import {
  commands,
  type ItemKind,
  type PackageSafety,
  type Scope,
} from "@/bindings";
import { marketKey } from "./marketplaces";

/** One offered package's identity across every marketplace query. */
export const safetyKey = (
  scope: Scope,
  source: string,
  kind: ItemKind,
  name: string,
): string => `${marketKey(scope, source)}::${kind}::${name}`;

interface PreinstallSafetyState {
  /** Answered scores; a key in flight or failed is simply absent, and the
   * dot stays quiet rather than guessing. */
  scores: Record<string, PackageSafety>;
  /** Queue a package's score. Fetches drain one at a time — a table of
   * forty rows must not fire forty scans at once; the backend caches, so a
   * revisit answers from disk. */
  want: (scope: Scope, source: string, kind: ItemKind, name: string) => void;
}

interface QueueItem {
  scope: Scope;
  source: string;
  kind: ItemKind;
  name: string;
  key: string;
}

const queue: QueueItem[] = [];
const queued = new Set<string>();
let draining = false;
// Bumped by a reset: an in-flight answer from before the bump is stale (the
// catalog may have moved) and is dropped instead of stored.
let generation = 0;

/** Empty the cache and the queue — called when any mutation can have moved
 * a catalog, so no score describes the commit before the change. */
export function resetPreinstallSafety() {
  generation += 1;
  queue.length = 0;
  queued.clear();
  usePreinstallSafety.setState({ scores: {} });
}

export const usePreinstallSafety = create<PreinstallSafetyState>(
  (set, get) => ({
    scores: {},
    want: (scope, source, kind, name) => {
      const key = safetyKey(scope, source, kind, name);
      if (get().scores[key] || queued.has(key)) return;
      queued.add(key);
      queue.push({ scope, source, kind, name, key });
      if (draining) return;
      draining = true;
      void (async () => {
        try {
          while (queue.length > 0) {
            const item = queue.shift();
            if (!item) break;
            const before = generation;
            try {
              const response = await commands.marketplacePackagePreview(
                item.scope,
                item.source,
                item.kind,
                item.name,
              );
              // A reset while this was in flight makes the answer stale.
              if (before !== generation) continue;
              if (response.status === "ok") {
                set((state) => ({
                  scores: {
                    ...state.scores,
                    [item.key]: response.data.safety,
                  },
                }));
              } else {
                // Retryable: the next mount of the row asks again instead
                // of showing "Checking…" forever.
                queued.delete(item.key);
              }
            } catch {
              // A transport error must not wedge the whole queue.
              queued.delete(item.key);
            }
          }
        } finally {
          draining = false;
        }
      })();
    },
  }),
);
