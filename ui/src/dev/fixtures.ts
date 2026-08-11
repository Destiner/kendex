import type {
  AppSettings,
  AuditView,
  DetectedHarness,
  Manifest_Serialize,
  ObservedItem,
  SourceRow,
} from "@/bindings";
import { manifests, sources, views } from "./fixture-declared";
import { harnesses, items } from "./fixture-observed";
import { ACME, API } from "./fixture-scopes";

export { ACME, API, AVAILABLE_SKILLS } from "./fixture-scopes";

export interface MockState {
  settings: AppSettings;
  harnesses: DetectedHarness[];
  items: ObservedItem[];
  missingProjects: string[];
  warnings: string[];
  views: AuditView[];
  manifests: Record<string, Manifest_Serialize>;
  sources: SourceRow[];
}

export function initialState(): MockState {
  return {
    settings: { schema: 1, projects: [ACME, API], appearance: "system" },
    harnesses: harnesses(),
    items: items(),
    missingProjects: [],
    warnings: [],
    views: views(),
    manifests: manifests(),
    sources: sources(),
  };
}
