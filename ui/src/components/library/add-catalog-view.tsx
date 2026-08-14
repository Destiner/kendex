import { Plus, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { Scope } from "@/bindings";
import { CatalogScopeGroup } from "@/components/catalog-scope";
import { AddCatalogDialog } from "@/components/library/add-catalog-dialog";
import { BundleGallery } from "@/components/library/bundle-gallery";
import { Button } from "@/components/ui/button";
import { scopeLabel } from "@/lib/derive";
import { sameScope } from "@/stores/audit";
import { useSettingsStore } from "@/stores/settings";
import { useSourcesStore } from "@/stores/sources";

/** "Add from a catalog": bundles lead, catalog management follows. */
export function AddCatalogView() {
  const {
    rows,
    bundles,
    busy,
    error,
    warnings,
    load,
    add,
    remove,
    toggle,
    installBundle,
    refreshRemotes,
  } = useSourcesStore();
  const projects = useSettingsStore((s) => s.settings?.projects ?? []);
  const [addOpen, setAddOpen] = useState(false);

  useEffect(() => {
    void load();
  }, [load]);

  const scopes: Scope[] = [
    { scope: "global" },
    ...projects.map((root): Scope => ({ scope: "project", root })),
  ];
  const hasBundles = bundles.length > 0;

  return (
    <div className="space-y-6 p-8">
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {warnings.map((w) => (
        <p key={w} className="text-sm text-muted-foreground">
          Heads up: {w}
        </p>
      ))}

      <section className="space-y-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-medium">Bundles</h2>
            <p className="text-sm text-muted-foreground">
              Curated sets from your catalogs — install everything in one go.
            </p>
          </div>
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => void refreshRemotes()}
          >
            <RefreshCw className="size-4" /> Check for updates
          </Button>
        </div>
        {hasBundles ? (
          scopes.map((scope) => (
            <BundleGallery
              key={scopeLabel(scope)}
              scope={scope}
              rows={bundles.filter((bundle) => sameScope(bundle.scope, scope))}
              busy={busy}
              onInstall={(source, name) =>
                void installBundle(scope, source, name)
              }
            />
          ))
        ) : (
          <p className="text-sm text-muted-foreground">
            No bundles yet — add a catalog below to see what it offers.
          </p>
        )}
      </section>

      <section className="space-y-3 border-t pt-6">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-sm font-medium">Catalogs</h2>
          <Button onClick={() => setAddOpen(true)}>
            <Plus className="size-4" /> Add a catalog
          </Button>
        </div>
        {scopes.map((scope) => (
          <CatalogScopeGroup
            key={scopeLabel(scope)}
            scope={scope}
            rows={rows.filter((row) => sameScope(row.scope, scope))}
            busy={busy}
            onToggle={(name, enabled) => void toggle(scope, name, enabled)}
            onRemove={(name) => void remove(scope, name)}
            onAddFocus={() => setAddOpen(true)}
          />
        ))}
      </section>

      <AddCatalogDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        scopes={scopes}
        busy={busy}
        onAdd={add}
      />
    </div>
  );
}
