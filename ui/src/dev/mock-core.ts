import type { AppSettings } from "@/bindings";
import { capabilityTable } from "./caps";
import { type Handler, label, store, view } from "./mock-state";

export const coreHandlers: Record<string, Handler> = {
  app_version: () => "0.1.0",
  capability_table: () => capabilityTable(),
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
};
