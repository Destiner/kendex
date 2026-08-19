// Reading marketplaces: the overview, a subscription's packages, a bundle,
// a package preview with its safety report, and the Library's provenance.
// Installing and subscribing live in mock-install.ts and mock-subscribe.ts.
import type { ItemKind, PackageView, Scope } from "@/bindings";
import { aboutViews } from "./fixture-marketplaces";
import { packageSafety } from "./fixture-package-safety";
import { bundleDetail, offeredHere } from "./mock-catalog";
import { installHandlers } from "./mock-install";
import { type Handler, store } from "./mock-state";
import { subscribeHandlers } from "./mock-subscribe";

export const marketplaceHandlers: Record<string, Handler> = {
  ...installHandlers,
  ...subscribeHandlers,

  marketplaces_overview: () => store.state.marketplaces,

  marketplace_packages: ({ scope, source }: { scope: Scope; source: string }) =>
    offeredHere(scope, source),

  marketplace_bundle: ({
    scope,
    source,
    name,
  }: {
    scope: Scope;
    source: string;
    name: string;
  }) => {
    const offered = offeredHere(scope, source);
    if (offered instanceof Promise) return offered;
    return bundleDetail(offered, source, name);
  },

  marketplace_package_preview: ({
    scope,
    source,
    kind,
    name,
  }: {
    scope: Scope;
    source: string;
    kind: ItemKind;
    name: string;
  }): PackageView | Promise<never> => {
    const offered = offeredHere(scope, source);
    if (offered instanceof Promise) return offered;
    const pkg = offered.find((p) => p.kind === kind && p.name === name);
    if (!pkg) {
      return Promise.reject(`'${name}' is not offered by '${source}'`);
    }
    return {
      preview: {
        kind: pkg.kind,
        name: pkg.name,
        description: pkg.description,
        tags: pkg.tags,
        readme:
          kind === "skill"
            ? `Use **${name}** for ${pkg.description?.toLowerCase()}.\n\nRead the checklist before acting.\n`
            : `# ${name}\n\n${pkg.description}\n`,
        files:
          kind === "skill"
            ? [
                { path: "SKILL.md", size: 1284, isReadme: false },
                { path: "README.md", size: 412, isReadme: true },
                { path: "checklist.md", size: 903, isReadme: false },
              ]
            : [
                {
                  path: `${name.split("/").at(-1)}.md`,
                  size: 764,
                  isReadme: false,
                },
              ],
        bundles: pkg.bundles,
        collision: pkg.collision,
      },
      safety: packageSafety(kind, name),
    };
  },

  marketplace_about: ({ scope, source }: { scope: Scope; source: string }) => {
    const offered = offeredHere(scope, source);
    if (offered instanceof Promise) return offered;
    const about = aboutViews[source];
    if (!about) {
      return Promise.reject(`source '${source}' is not fetched yet`);
    }
    return about;
  },

  library_provenance: () => store.state.provenance,
};
