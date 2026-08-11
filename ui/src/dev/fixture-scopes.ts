import type { Scope } from "@/bindings";

export const ACME = "/home/dana/work/acme-web";
export const API = "/home/dana/work/api-server";

export const AVAILABLE_SKILLS = [
  "code-review",
  "deploy",
  "docs",
  "github",
  "release-notes",
  "tests",
];

export const GLOBAL: Scope = { scope: "global" };
export const proj = (root: string): Scope => ({ scope: "project", root });
