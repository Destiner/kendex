import type { AppSettings, Scope } from "@/bindings";
import { capabilityTable } from "./caps";
import { type Handler, label, same, store, view } from "./mock-state";

export const coreHandlers: Record<string, Handler> = {
  app_version: () => "0.1.0",
  capability_table: () => capabilityTable(),
  // No real window to act on in the mock browser harness.
  window_minimize: () => null,
  window_toggle_maximize: () => null,
  window_close: () => null,
  scan_machine: () => ({
    harnesses: store.state.harnesses,
    items: store.state.items,
    missingProjects: store.state.missingProjects,
    warnings: store.state.warnings,
  }),
  get_settings: () => store.state.settings,
  update_settings: ({ settings }: { settings: AppSettings }) => {
    store.state.settings = settings;
    return store.state.settings;
  },
  register_project: ({ path }: { path: string }) => {
    const projects = store.state.settings.projects ?? [];
    if (!projects.includes(path)) {
      store.state.settings.projects = [...projects, path];
    }
    view({ scope: "project", root: path });
    return store.state.settings;
  },
  unregister_project: ({ path }: { path: string }) => {
    store.state.settings.projects = (
      store.state.settings.projects ?? []
    ).filter((p) => p !== path);
    store.state.views = store.state.views.filter(
      (v) => label(v.scope) !== path,
    );
    return store.state.settings;
  },
  discover_projects: ({ root }: { root: string }) =>
    ["acme-web", "api-server", "demo-app"].map(
      (name) => `${root.replace(/\/+$/, "")}/${name}`,
    ),
  report_route: ({ scope, name }: { scope: Scope; name: string }) => {
    const upstream = "vanillagreencom/vstack";
    // Mirrors the engine's rule: skills never route upstream through
    // provenance alone, everything else from the catalog does.
    const owned = store.state.items.some(
      (it) =>
        it.name === name &&
        same(it.scope, scope) &&
        it.kind !== "skill" &&
        it.origin === upstream,
    );
    const kind = store.state.items.find((it) => it.name === name)?.kind;
    const label = !owned
      ? null
      : kind === "hook" || kind === "pi-extension"
        ? "harness"
        : kind === "agent"
          ? "skills"
          : "cli";
    return {
      vstackOwned: owned,
      repo: owned ? upstream : null,
      label,
      issueUrl: owned
        ? `https://github.com/${upstream}/issues/new?title=${encodeURIComponent(`${name}: `)}${label ? `&labels=${label}` : ""}`
        : null,
    };
  },
};
