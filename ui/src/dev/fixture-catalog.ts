// What the two subscribed catalogs offer: the kendex catalog's items and
// the plugin registry's, whose names are plugin/item, plus the bundles each
// groups them into. The subscriptions themselves live next door in
// fixture-marketplaces.ts.
import type { ItemKind, Tag } from "@/bindings";

export const KENDEX_REPO = "vanillagreencom/kendex";
export const PLUGINS_REPO = "acme/claude-plugins";
export const KENDEX_HEAD = "9f31c2a";
export const PLUGINS_HEAD = "4c1d9e2";

export interface Offered {
  kind: ItemKind;
  name: string;
  description: string;
  tags: Tag[];
  bundles: string[];
}

export const KENDEX_OFFERED: Offered[] = [
  {
    kind: "agent",
    name: "orch",
    description: "Coordinates multi-step work across agents",
    tags: ["automation", "planning"],
    bundles: ["starter"],
  },
  {
    kind: "agent",
    name: "reviewer",
    description: "Reviews changes before they merge",
    tags: ["review"],
    bundles: ["review"],
  },
  {
    kind: "skill",
    name: "github",
    description: "Work with GitHub from the terminal",
    tags: ["git"],
    bundles: ["starter", "platform"],
  },
  {
    kind: "skill",
    name: "deploy",
    description: "Ship to staging and production safely",
    tags: ["release"],
    bundles: ["starter"],
  },
  {
    kind: "skill",
    name: "code-review",
    description: "A structured checklist for reviewing changes",
    tags: ["review"],
    bundles: ["review"],
  },
  {
    kind: "skill",
    name: "docs",
    description: "Write and maintain project documentation",
    tags: ["docs"],
    bundles: ["platform"],
  },
  {
    kind: "skill",
    name: "release-notes",
    description: "Turn merged work into release notes",
    tags: ["release", "docs"],
    bundles: ["platform"],
  },
  {
    kind: "skill",
    name: "tests",
    description: "Write and run tests the project's way",
    tags: ["testing"],
    bundles: ["platform"],
  },
  {
    kind: "skill",
    name: "webhook-relay",
    description: "Forward repository events to a chat channel",
    tags: ["integration"],
    bundles: [],
  },
  {
    kind: "hook",
    name: "guard",
    description: "Runs checks before every commit",
    tags: ["security"],
    bundles: [],
  },
  {
    kind: "command",
    name: "ship-it",
    description: "Draft a release pull request from the current branch",
    tags: ["release"],
    bundles: ["starter", "platform"],
  },
  {
    kind: "mcp-server",
    name: "postgres",
    description: "Query the app database from the assistant",
    tags: ["data"],
    bundles: ["platform"],
  },
];

export const PLUGINS_OFFERED: Offered[] = [
  {
    kind: "agent",
    name: "deploy-kit/release-manager",
    description: "Runs a release from tag to announcement",
    tags: ["release"],
    bundles: ["deploy-kit"],
  },
  {
    kind: "command",
    name: "deploy-kit/rollback",
    description: "Roll the last deploy back in one step",
    tags: ["release"],
    bundles: ["deploy-kit"],
  },
  {
    kind: "agent",
    name: "docs-kit/writer",
    description: "Drafts documentation from code and commits",
    tags: ["docs"],
    bundles: ["docs-kit"],
  },
  {
    kind: "command",
    name: "docs-kit/outline",
    description: "Sketch a document before writing it",
    tags: ["docs", "planning"],
    bundles: ["docs-kit"],
  },
  {
    kind: "skill",
    name: "docs-kit/style-guide",
    description: "The house style for prose and headings",
    tags: ["docs"],
    bundles: ["docs-kit"],
  },
];

export interface BundleSpec {
  description: string;
  version: string | null;
  category: string | null;
  members: { kind: ItemKind; name: string }[];
}

export const BUNDLE_SPECS: Record<string, Record<string, BundleSpec>> = {
  kendex: {
    starter: {
      description: "Everything a new repo needs",
      version: null,
      category: null,
      members: [
        { kind: "agent", name: "orch" },
        { kind: "skill", name: "github" },
        { kind: "skill", name: "deploy" },
        { kind: "command", name: "ship-it" },
      ],
    },
    review: {
      description: "Code review, end to end",
      version: "1.2.0",
      category: "quality",
      members: [
        { kind: "agent", name: "reviewer" },
        { kind: "skill", name: "code-review" },
      ],
    },
    platform: {
      description: "The full platform workflow, docs to deploy",
      version: "0.9.0",
      category: "workflow",
      members: [
        { kind: "skill", name: "github" },
        { kind: "skill", name: "docs" },
        { kind: "skill", name: "tests" },
        { kind: "skill", name: "release-notes" },
        { kind: "command", name: "ship-it" },
        { kind: "mcp-server", name: "postgres" },
      ],
    },
  },
  "claude-plugins": {
    "deploy-kit": {
      description: "Release and rollback, as one set",
      version: "2.1.0",
      category: null,
      members: [
        { kind: "agent", name: "deploy-kit/release-manager" },
        { kind: "command", name: "deploy-kit/rollback" },
      ],
    },
    "docs-kit": {
      description: "Documentation, outlined and styled",
      version: "1.0.3",
      category: null,
      members: [
        { kind: "agent", name: "docs-kit/writer" },
        { kind: "command", name: "docs-kit/outline" },
        { kind: "skill", name: "docs-kit/style-guide" },
      ],
    },
  },
};
