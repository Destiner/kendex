import { toast } from "sonner";
import { create } from "zustand";
import type { MarketplaceRow } from "@/bindings";
import {
  type AboutView,
  type AvailablePackage,
  type BundleDetail,
  commands,
  type InstallItem,
  type Scope,
} from "@/bindings";
import { useAuditStore } from "./audit";
import { useScanStore } from "./scan";

/** One subscription's cache key: where it lives plus its alias. */
export const marketKey = (scope: Scope, source: string): string =>
  `${scope.scope === "global" ? "global" : scope.root}::${source}`;

interface MarketplacesState {
  rows: MarketplaceRow[];
  /** Each opened subscription's offered packages, by [marketKey]. */
  packages: Record<string, AvailablePackage[]>;
  /** Each opened subscription's About report, by [marketKey]. */
  about: Record<string, AboutView>;
  /** Each opened curated set, by [marketKey]::bundle. */
  bundles: Record<string, BundleDetail>;
  loaded: boolean;
  busy: boolean;
  error: string | null;
  load: () => Promise<void>;
  loadPackages: (scope: Scope, source: string) => Promise<void>;
  loadAbout: (scope: Scope, source: string) => Promise<void>;
  loadBundle: (scope: Scope, source: string, name: string) => Promise<void>;
  subscribe: (
    scope: Scope,
    reference: string,
    name: string | null,
  ) => Promise<boolean>;
  unsubscribe: (
    scope: Scope,
    source: string,
    keep: boolean,
    discardEdits: boolean,
  ) => Promise<boolean>;
  toggle: (scope: Scope, source: string, enabled: boolean) => Promise<void>;
  checkForUpdates: () => Promise<void>;
  install: (opts: {
    scope: Scope;
    source: string;
    items: InstallItem[];
    bundle?: string | null;
    destination?: Scope | null;
  }) => Promise<boolean>;
}

/** What lands after any mutation: the tables everywhere else stay current. */
async function refreshDownstream() {
  await useScanStore.getState().refresh();
  await useAuditStore.getState().refresh();
}

export const useMarketplacesStore = create<MarketplacesState>((set, get) => ({
  rows: [],
  packages: {},
  about: {},
  bundles: {},
  loaded: false,
  busy: false,
  error: null,

  load: async () => {
    const response = await commands.marketplacesOverview();
    if (response.status === "ok") {
      set({ rows: response.data, loaded: true, error: null });
    } else {
      set({ loaded: true, error: response.error });
    }
  },

  loadPackages: async (scope, source) => {
    const response = await commands.marketplacePackages(scope, source);
    if (response.status === "ok") {
      set((state) => ({
        packages: {
          ...state.packages,
          [marketKey(scope, source)]: response.data,
        },
      }));
    }
  },

  loadAbout: async (scope, source) => {
    const response = await commands.marketplaceAbout(scope, source);
    if (response.status === "ok") {
      set((state) => ({
        about: { ...state.about, [marketKey(scope, source)]: response.data },
      }));
    }
  },

  loadBundle: async (scope, source, name) => {
    const response = await commands.marketplaceBundle(scope, source, name);
    if (response.status === "ok") {
      set((state) => ({
        bundles: {
          ...state.bundles,
          [`${marketKey(scope, source)}::${name}`]: response.data,
        },
      }));
    }
  },

  subscribe: async (scope, reference, name) => {
    set({ busy: true });
    let response: Awaited<ReturnType<typeof commands.marketplaceSubscribe>>;
    try {
      response = await commands.marketplaceSubscribe(scope, reference, name);
    } finally {
      set({ busy: false });
    }
    if (response.status === "error") {
      // The dialog shows the refusal beside the input; no toast on top.
      set({ error: response.error });
      return false;
    }
    set({ error: null });
    toast.success(`Subscribed to '${response.data.name}'`);
    for (const note of response.data.notes) toast.message(note);
    await get().load();
    return true;
  },

  unsubscribe: async (scope, source, keep, discardEdits) => {
    set({ busy: true });
    let response: Awaited<ReturnType<typeof commands.marketplaceUnsubscribe>>;
    try {
      response = await commands.marketplaceUnsubscribe(
        scope,
        source,
        keep,
        discardEdits,
      );
    } finally {
      set({ busy: false });
    }
    if (response.status === "error") {
      set({ error: response.error });
      return false;
    }
    set({ error: null });
    toast.success(
      keep
        ? `Unsubscribed from '${source}' — its packages are yours now`
        : `Unsubscribed from '${source}'`,
    );
    await get().load();
    await refreshDownstream();
    return true;
  },

  toggle: async (scope, source, enabled) => {
    const response = await commands.sourceToggle(scope, source, enabled);
    if (response.status === "error") {
      toast.error(response.error);
      return;
    }
    await get().load();
    await refreshDownstream();
  },

  checkForUpdates: async () => {
    set({ busy: true });
    try {
      const response = await commands.sourcesRefresh();
      if (response.status === "ok") {
        for (const warning of response.data) toast.message(warning);
        await get().load();
      } else {
        toast.error(response.error);
      }
    } finally {
      set({ busy: false });
    }
  },

  install: async ({ scope, source, items, bundle = null, destination }) => {
    set({ busy: true });
    let response: Awaited<ReturnType<typeof commands.marketplaceInstall>>;
    try {
      response = await commands.marketplaceInstall(
        scope,
        source,
        items,
        bundle,
        destination ?? null,
        false,
      );
    } finally {
      set({ busy: false });
    }
    if (response.status === "error") {
      toast.error(response.error);
      return false;
    }
    // The command answers with the refreshed package list for this
    // subscription, so the table flips to Installed without a second query.
    const key = marketKey(destination ?? scope, source);
    set((state) => ({
      packages: { ...state.packages, [key]: response.data },
      error: null,
    }));
    const what = bundle
      ? `the ${bundle} bundle`
      : items.length === 1
        ? items[0].name
        : `${items.length} packages`;
    toast.success(`Installed ${what}`);
    await refreshDownstream();
    return true;
  },
}));
