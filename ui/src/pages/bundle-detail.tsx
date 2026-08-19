import { useEffect, useState } from "react";
import type { Scope } from "@/bindings";
import { DestinationSelect } from "@/components/marketplaces/destination-select";
import { PageHeader } from "@/components/page-header";
import { StatusDot } from "@/components/status-dot";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { kindIcon } from "@/lib/kind-icon";
import { kindLabel, packageDisplayName } from "@/lib/labels";
import { CONTENT_WIDTH, PAGE_BODY } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { marketKey, useMarketplacesStore } from "@/stores/marketplaces";
import { useNavStore } from "@/stores/nav";

/** One curated set: install the whole thing as a set that keeps itself
 * whole, or pick members to install as your own choices. Both go through
 * the normal preview and safety gate. */
export function BundleDetailPage() {
  const bundleRef = useNavStore((s) => s.bundleRef);
  const bundles = useMarketplacesStore((s) => s.bundles);
  const loadBundle = useMarketplacesStore((s) => s.loadBundle);
  const install = useMarketplacesStore((s) => s.install);
  const busy = useMarketplacesStore((s) => s.busy);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [destination, setDestination] = useState<Scope | null>(null);

  useEffect(() => {
    if (!bundleRef) return;
    void loadBundle(bundleRef.scope, bundleRef.source, bundleRef.bundle);
  }, [bundleRef, loadBundle]);

  if (!bundleRef) return null;
  const { scope, source, bundle } = bundleRef;
  const detail = bundles[`${marketKey(scope, source)}::${bundle}`];
  const target = destination ?? scope;
  const redirected = target !== scope ? target : null;

  const memberKey = (kind: string, name: string) => `${kind}:${name}`;
  const toggleMember = (kind: string, name: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      const key = memberKey(kind, name);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const installAll = () =>
    void install({ scope, source, items: [], bundle, destination: redirected });
  const installSelected = () => {
    if (!detail) return;
    const items = detail.members
      .filter((m) => selected.has(memberKey(m.kind, m.name)))
      .map((m) => ({ kind: m.kind, name: m.name }));
    if (items.length === 0) return;
    void install({ scope, source, items, destination: redirected }).then(
      (ok) => {
        if (ok) setSelected(new Set());
      },
    );
  };

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title={bundle}
        subtitle={
          detail ? (
            <>
              {detail.description ? <p>{detail.description}</p> : null}
              <p className="mt-1 text-xs">
                {[detail.version ? `v${detail.version}` : null, source]
                  .filter(Boolean)
                  .join(" · ")}
              </p>
            </>
          ) : null
        }
        action={
          <Button disabled={busy || !detail} onClick={installAll}>
            Install all
          </Button>
        }
      />
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className={cn(PAGE_BODY, "pt-0")}>
          <div className={CONTENT_WIDTH}>
            {!detail ? (
              <p className="py-16 text-center text-sm text-muted-foreground">
                Reading the set…
              </p>
            ) : (
              <>
                <div className="divide-y rounded-lg border">
                  {detail.members.map((member) => {
                    const Icon = kindIcon(member.kind);
                    const installable = member.state === "available";
                    const id = `member-${memberKey(member.kind, member.name)}`;
                    return (
                      <label
                        key={memberKey(member.kind, member.name)}
                        htmlFor={id}
                        className={cn(
                          "flex items-center gap-3 px-4 py-2.5",
                          installable ? "cursor-pointer" : "opacity-80",
                        )}
                      >
                        <Checkbox
                          id={id}
                          checked={selected.has(
                            memberKey(member.kind, member.name),
                          )}
                          disabled={!installable}
                          onCheckedChange={() =>
                            toggleMember(member.kind, member.name)
                          }
                        />
                        <Icon className="size-4 shrink-0 text-muted-foreground" />
                        <span className="min-w-0 flex-1 truncate font-medium">
                          {packageDisplayName(member)}
                        </span>
                        <span className="w-24 text-xs text-muted-foreground">
                          {kindLabel(member.kind)}
                        </span>
                        <span className="w-32 text-right text-xs">
                          {member.state === "installed" ? (
                            <span className="text-muted-foreground">
                              Installed
                            </span>
                          ) : member.state === "held-back-by-safety" ? (
                            <span className="text-warning">Held back</span>
                          ) : member.state === "not-offered" ? (
                            <span className="text-muted-foreground">
                              No longer offered
                            </span>
                          ) : (
                            <StatusDot
                              tone="good"
                              className="inline-block"
                              title="Available"
                            />
                          )}
                        </span>
                      </label>
                    );
                  })}
                </div>
                <div className="mt-4 flex items-center justify-end gap-2">
                  <DestinationSelect
                    browsing={scope}
                    value={target}
                    onChange={setDestination}
                  />
                  <Button
                    variant="outline"
                    disabled={busy || selected.size === 0}
                    onClick={installSelected}
                  >
                    Install {selected.size} selected
                  </Button>
                </div>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
