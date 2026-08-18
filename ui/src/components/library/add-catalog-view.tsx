import { Boxes } from "lucide-react";
import { useEffect, useState } from "react";
import type { Scope } from "@/bindings";
import { CatalogScopeGroup } from "@/components/catalog-scope";
import { EmptyState } from "@/components/empty-state";
import { AddCatalogDialog } from "@/components/library/add-catalog-dialog";
import { BundleGallery } from "@/components/library/bundle-gallery";
import { Section } from "@/components/section";
import { StatusNote } from "@/components/status-note";
import { Button } from "@/components/ui/button";
import {
  ADD_CATALOG_LABEL,
  BUNDLES_HELP,
  CATALOGS_HELP,
  CHECK_UPDATES_LABEL,
  NO_CATALOGS_TITLE,
  NO_CATALOGS_YET,
} from "@/lib/copy";
import { scopeLabel } from "@/lib/derive";
import { PAGE_BODY, WIDE_CONTENT_WIDTH } from "@/lib/layout";
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
    <div className={cn("space-y-8", WIDE_CONTENT_WIDTH, PAGE_BODY)}>
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

      {/* Nothing to install from is one situation, not two empty sections:
          bundles come out of catalogs, so with no catalog there is nothing
          to say about them. */}
      {hasCatalogs ? (
        <>
          {hasBundles ? (
            <Section
              title="Bundles"
              description={BUNDLES_HELP}
              action={
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={busy}
                  onClick={() => void refreshRemotes()}
                >
                  {CHECK_UPDATES_LABEL}
                </Button>
              }
            >
              <div className="space-y-3">
                {scopes.map((scope) => (
                  <BundleGallery
                    key={scopeLabel(scope)}
                    scope={scope}
                    rows={bundles.filter((bundle) =>
                      sameScope(bundle.scope, scope),
                    )}
                    busy={busy}
                    onInstall={(source, name) =>
                      void installBundle(scope, source, name)
                    }
                  />
                ))}
              </div>
            </Section>
          ) : null}

          <Section
            title="Catalogs"
            description={CATALOGS_HELP}
            action={
              <Button size="sm" onClick={() => setAddOpen(true)}>
                {ADD_CATALOG_LABEL}
              </Button>
            }
          >
            <div className="space-y-3">
              {scopes.map((scope) => (
                <CatalogScopeGroup
                  key={scopeLabel(scope)}
                  scope={scope}
                  rows={rows.filter((row) => sameScope(row.scope, scope))}
                  busy={busy}
                  onToggle={(name, enabled) =>
                    void toggle(scope, name, enabled)
                  }
                  onRemove={(name) => void remove(scope, name)}
                />
              ))}
            </div>
          </Section>
        </>
      ) : (
        <EmptyState
          icon={Boxes}
          title={NO_CATALOGS_TITLE}
          action={
            <Button onClick={() => setAddOpen(true)}>
              {ADD_CATALOG_LABEL}
            </Button>
          }
        >
          {NO_CATALOGS_YET}
        </EmptyState>
      )}

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
