// Product prose for Customize: the page's own words, the editor's tabs, and
// the states where nothing has been written yet. Same house style as
// copy.ts — split out for the file line cap.

// Customize → Agent settings. What a value here does, and the one case
// where it does nothing at all.
export const FRONTMATTER_HELP =
  "Your value wins over the catalog's. Leave a field blank to keep the catalog's.";
export const FRONTMATTER_IGNORED = (tool: string): string =>
  `${tool} doesn't read agent settings — anything saved here is kept, but has no effect.`;
export const NO_AGENTS_YET = "No agents installed here";
export const NO_AGENTS_YET_BODY =
  "Install an agent from a catalog, then its settings show up here.";

// Customize: what the page is for, and the state where nothing has been
// written yet.
export const CUSTOMIZE_SUBTITLE = "Your own edits on top of what you installed";
export const NOTHING_CUSTOMIZED = "Nothing customized here yet";
export const NOTHING_CUSTOMIZED_BODY =
  "Add your own instructions, agent settings and hooks, and vstack writes them into every tool.";
export const START_CUSTOMIZING = "Start customizing";
