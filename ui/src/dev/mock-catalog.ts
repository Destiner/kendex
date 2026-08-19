// Reading a subscription's catalog: the lookups the marketplace handlers
// share, including the rejection that says why content is unreachable.
import type {
  AvailablePackage,
  BundleDetail,
  InstallState,
  ItemKind,
  MarketplaceRow,
  Scope,
} from "@/bindings";
import { BUNDLE_SPECS } from "./fixture-catalog";
import { packagesKey } from "./fixture-marketplaces";
import { same, store } from "./mock-state";

export function marketplaceRow(
  scope: Scope,
  source: string,
): MarketplaceRow | undefined {
  return store.state.marketplaces.find(
    (row) => row.name === source && same(row.scope, scope),
  );
}

/// The readable catalog's packages, or the rejection that says why the
/// content is unreachable — mirrors core's require_ready errors.
export function offeredHere(
  scope: Scope,
  source: string,
): AvailablePackage[] | Promise<never> {
  if (!marketplaceRow(scope, source)) {
    return Promise.reject(`unknown source '${source}'`);
  }
  const offered = store.state.marketplacePackages[packagesKey(scope, source)];
  if (!offered) {
    return Promise.reject(`source '${source}' is not fetched yet`);
  }
  return offered;
}

const stateOf = (
  offered: AvailablePackage[],
  kind: ItemKind,
  name: string,
): InstallState =>
  offered.find((pkg) => pkg.kind === kind && pkg.name === name)?.state ??
  "available";

export function bundleDetail(
  offered: AvailablePackage[],
  source: string,
  name: string,
): BundleDetail | Promise<never> {
  const spec = BUNDLE_SPECS[source]?.[name];
  if (!spec) {
    return Promise.reject(`no bundle named '${name}' in '${source}'`);
  }
  const members = spec.members.map((member) => ({
    kind: member.kind,
    name: member.name,
    state: stateOf(offered, member.kind, member.name),
  }));
  return {
    name,
    description: spec.description,
    version: spec.version,
    category: spec.category,
    members,
    installedMembers: members.filter((m) => m.state === "installed").length,
    totalMembers: members.length,
    collision: null,
  };
}
