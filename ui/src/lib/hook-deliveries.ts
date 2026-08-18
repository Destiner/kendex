import { useEffect, useState } from "react";
import { commands, type HookDelivery, type Scope } from "@/bindings";
import type { DraftHook } from "@/lib/editor-draft";

/** Where each drafted hook would land, asked of the same decision the
 *  engine installs by — the line under a hook can then never disagree with
 *  what saving does. Debounced behind typing; compared by content, so only
 *  a real edit re-asks. */
export function useHookDeliveries(
  scope: Scope,
  hooks: DraftHook[],
): HookDelivery[][] {
  const [deliveries, setDeliveries] = useState<HookDelivery[][]>([]);
  const drafted = JSON.stringify(hooks);
  useEffect(() => {
    const hooks = JSON.parse(drafted) as DraftHook[];
    if (hooks.length === 0) {
      setDeliveries([]);
      return;
    }
    let stale = false;
    const timer = setTimeout(() => {
      void commands.customHookDeliveries(scope, hooks).then((result) => {
        if (!stale && result.status === "ok") setDeliveries(result.data);
      });
    }, 250);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  }, [drafted, scope]);
  return deliveries;
}
