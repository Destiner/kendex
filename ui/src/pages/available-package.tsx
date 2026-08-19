import { useEffect, useState } from "react";
import { commands, type PackageView, type Scope } from "@/bindings";
import { MarkdownView } from "@/components/markdown-view";
import { DestinationSelect } from "@/components/marketplaces/destination-select";
import { fileSizeLabel } from "@/components/package/file-list";
import { PageHeader } from "@/components/page-header";
import { FindingLine } from "@/components/safety-findings";
import { TagBadges } from "@/components/tag-badge";
import { Button } from "@/components/ui/button";
import { kindIcon } from "@/lib/kind-icon";
import { kindLabel, packageDisplayName, VERDICT_LABELS } from "@/lib/labels";
import { PAGE_BODY, WIDE_CONTENT_WIDTH } from "@/lib/layout";
import { cn } from "@/lib/utils";
import { marketKey, useMarketplacesStore } from "@/stores/marketplaces";
import { useNavStore } from "@/stores/nav";

/** A package that isn't installed yet: what it is, its own README, its
 * files, and the safety findings its bytes earn before anything lands.
 * Installing turns this same address into the installed package's page. */
export function AvailablePackagePage() {
  const availableRef = useNavStore((s) => s.availableRef);
  const goToPackage = useNavStore((s) => s.goToPackage);
  const rows = useMarketplacesStore((s) => s.rows);
  const install = useMarketplacesStore((s) => s.install);
  const busy = useMarketplacesStore((s) => s.busy);
  const [view, setView] = useState<PackageView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [destination, setDestination] = useState<Scope | null>(null);

  useEffect(() => {
    if (!availableRef) return;
    setView(null);
    setError(null);
    void commands
      .marketplacePackagePreview(
        availableRef.scope,
        availableRef.source,
        availableRef.kind,
        availableRef.name,
      )
      .then((r) => {
        if (r.status === "ok") setView(r.data);
        else setError(r.error);
      });
  }, [availableRef]);

  if (!availableRef) return null;
  const { scope, source, kind, name } = availableRef;
  const Icon = kindIcon(kind);
  const target = destination ?? scope;
  // Matched by scope and name both — two scopes can subscribe the same
  // alias to different repositories.
  const row = rows.find(
    (r) =>
      r.name === source &&
      marketKey(r.scope, r.name) === marketKey(scope, source),
  );
  const repo = row?.repo ?? row?.path ?? null;

  const doInstall = () =>
    void install({
      scope,
      source,
      items: [{ kind, name }],
      destination: target !== scope ? target : null,
    }).then((ok) => {
      // Installed, the same page carries on in its installed mode — the
      // address gains the scope it landed in.
      if (ok) goToPackage({ kind, name, scope: target });
    });

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        wide
        title={
          <span className="flex items-center gap-2.5">
            <Icon className="size-6 text-muted-foreground" />
            {packageDisplayName({ kind, name })}
          </span>
        }
        subtitle={
          <>
            {view?.preview.description ? (
              <p>{view.preview.description}</p>
            ) : null}
            <span className="mt-1 flex items-center gap-2">
              <span className="text-xs">{kindLabel(kind)}</span>
              <TagBadges tags={view?.preview.tags ?? []} />
            </span>
          </>
        }
        action={
          <>
            <DestinationSelect
              browsing={scope}
              value={target}
              onChange={setDestination}
            />
            <Button disabled={busy || !view} onClick={doInstall}>
              {busy ? "Installing…" : "Install"}
            </Button>
          </>
        }
      />
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className={cn(PAGE_BODY, "pt-0")}>
          <div
            className={cn(
              WIDE_CONTENT_WIDTH,
              "grid gap-8 lg:grid-cols-[minmax(0,1fr)_20rem]",
            )}
          >
            <div className="min-w-0 space-y-8">
              {error ? (
                <p className="text-sm text-critical" role="alert">
                  {error}
                </p>
              ) : null}
              {view?.preview.readme ? (
                <section>
                  <MarkdownView source={view.preview.readme} />
                </section>
              ) : view && !error ? (
                <p className="text-sm text-muted-foreground">
                  This package carries no README.
                </p>
              ) : null}
              {view && view.safety.findings.length > 0 ? (
                <section>
                  <h3 className="mb-3 text-sm font-semibold">
                    Before you install
                  </h3>
                  <div className="space-y-3">
                    {view.safety.findings.map((finding) => (
                      <FindingLine
                        key={`${finding.location}:${finding.message}`}
                        finding={finding}
                      />
                    ))}
                  </div>
                </section>
              ) : null}
            </div>
            <aside className="space-y-6 text-sm">
              <section>
                <h3 className="mb-1 text-xs font-semibold text-muted-foreground uppercase">
                  From
                </h3>
                <p>
                  {source}
                  {repo ? (
                    <span className="block truncate font-mono text-xs text-muted-foreground">
                      {repo}
                    </span>
                  ) : null}
                </p>
              </section>
              {view ? (
                <section>
                  <h3 className="mb-1 text-xs font-semibold text-muted-foreground uppercase">
                    Safety
                  </h3>
                  <p>
                    {VERDICT_LABELS[view.safety.verdict]} ·{" "}
                    {view.safety.safety.score}/100
                  </p>
                </section>
              ) : null}
              {view && view.preview.bundles.length > 0 ? (
                <section>
                  <h3 className="mb-1 text-xs font-semibold text-muted-foreground uppercase">
                    Comes with
                  </h3>
                  <p>{view.preview.bundles.join(", ")}</p>
                </section>
              ) : null}
              {view && view.preview.files.length > 0 ? (
                <section>
                  <h3 className="mb-1 text-xs font-semibold text-muted-foreground uppercase">
                    Files
                  </h3>
                  <ul className="space-y-1">
                    {view.preview.files.map((file) => (
                      <li
                        key={file.path}
                        className="flex items-baseline justify-between gap-2"
                      >
                        <span className="truncate font-mono text-xs">
                          {file.path}
                        </span>
                        <span className="shrink-0 text-xs text-muted-foreground">
                          {fileSizeLabel(file.size)}
                        </span>
                      </li>
                    ))}
                  </ul>
                </section>
              ) : null}
              {view?.preview.collision ? (
                <p className="text-xs text-warning">
                  This name is already installed from {view.preview.collision}—
                  installing from {source} will be refused.
                </p>
              ) : null}
            </aside>
          </div>
        </div>
      </div>
    </div>
  );
}
