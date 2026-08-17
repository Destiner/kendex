import { Plus, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { Scope } from "@/bindings";
import { CatalogScopeGroup } from "@/components/catalog-scope";
import { AddCatalogDialog } from "@/components/library/add-catalog-dialog";
import { BundleGallery } from "@/components/library/bundle-gallery";
import { SectionHeading } from "@/components/section";
import { StatusNote } from "@/components/status-note";
import { Button } from "@/components/ui/button";
import {
  BUNDLES_HELP,
  CATALOGS_HELP,
  NO_BUNDLES_YET,
  NO_CATALOGS_YET,
} from "@/lib/copy";
import { scopeLabel } from "@/lib/derive";
import { CONTENT_WIDTH, PAGE_BODY } from "@/lib/layout";
import { cn } from "@/lib/utils";
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
  const hasCatalogs = rows.length > 0;

  return (
    <div className={cn("space-y-6", CONTENT_WIDTH, PAGE_BODY)}>
      {error ? (
        <StatusNote tone="critical" title="That catalog couldn't be added">
          {error}
        </StatusNote>
      ) : null}
      {warnings.map((w) => (
        <StatusNote key={w} tone="warning" title="Heads up">
          {w}
        </StatusNote>
      ))}

      <section className="space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div>
            <SectionHeading>Bundles</SectionHeading>
            <p className="text-sm text-muted-foreground">{BUNDLES_HELP}</p>
          </div>
          <Button
            variant="outline"
            className="shrink-0"
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
          <p className="text-sm text-muted-foreground">{NO_BUNDLES_YET}</p>
        )}
      </section>

      <section className="space-y-3 border-t pt-6">
        <div className="flex items-start justify-between gap-3">
          <div>
            <SectionHeading>Catalogs</SectionHeading>
            <p className="text-sm text-muted-foreground">{CATALOGS_HELP}</p>
          </div>
          <Button className="shrink-0" onClick={() => setAddOpen(true)}>
            <Plus className="size-4" /> Add a catalog
          </Button>
        </div>
        {hasCatalogs ? (
          scopes.map((scope) => (
            <CatalogScopeGroup
              key={scopeLabel(scope)}
              scope={scope}
              rows={rows.filter((row) => sameScope(row.scope, scope))}
              busy={busy}
              onToggle={(name, enabled) => void toggle(scope, name, enabled)}
              onRemove={(name) => void remove(scope, name)}
            />
          ))
        ) : (
          <p className="text-sm text-muted-foreground">{NO_CATALOGS_YET}</p>
        )}
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
