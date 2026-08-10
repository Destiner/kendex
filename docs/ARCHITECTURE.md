# vstack2 Architecture

Cross-platform desktop app (Rust + Tauri) managing AI coding-harness
customizations — agents, skills, hooks, commands, MCP servers, plugins, Pi
extensions — across global and per-project scopes. Claude Code first-class;
codex, opencode, cursor, and pi behind the same adapter seam. No server; a
thin CLI mirrors every core operation so consuming-repo automation
(refresh, report, …) keeps working.

## The one idea

Four verbs over one model: **scan → declare → diff → apply**.

- **Scan** — read harness-native directories in place, across all scopes.
  Useful read-only with zero adoption; nothing copies into a shadow store.
- **Declare** — a per-scope `vstack.toml` manifest is the only durable home
  of user intent.
- **Diff** — drift = declared vs observed. The Audit page is this diff.
- **Apply** — make disk match declaration, plan shown first. Adopt is the
  reverse arrow: record an observed item into the manifest.

Every page and every CLI verb is a projection of these four; none owns
logic.

## Vocabulary

Scope (global | project) · Harness (adapter + capability table) · ItemKind
(agent, skill, hook, command, mcp-server, plugin, pi-extension) ·
Item (logical: kind + name from a source) · Installation (item × harness ×
scope — what locks, drift rows, and applies track) · Source (path | git;
registry reserved post-release) · Manifest · Lock (provenance + hash) ·
Observation (scanner truth) · Drift. Core modules mirror the verbs: `model`, `scan`,
`manifest`, `diff`, `apply`, `source`, `harness/` (one file per harness).

## Layout

`crates/core` — pure domain. `crates/app` — Tauri commands, one module per
page domain; events stream scan progress. `crates/cli` — thin verbs over
the same core. `ui/` — React 19 + Tailwind v4 +
shadcn/ui + zustand over generated bindings (tauri-specta). Adapters in
`core/harness/` own paths and rendering only; what each harness supports
lives in one capability table read by core and UI.

## Invariants — what the product guarantees

1. Generated artifacts are always overwritable; refresh regenerates from
   scratch and re-merges the manifest.
2. Write-only-if-absent: never clobber a user-set value; never re-add a
   user removal. This protects manifest values and unrelated
   structured-config keys — managed generated content is replaceable
   (invariant 1); the two never overlap.
3. Content hashes cover source bytes plus the manifest sections that shape
   an artifact — editing a shared key invalidates dependents.
4. Locks record durable provenance; same-source reinstall is a no-op,
   cross-source name collision is a hard error naming the original.
5. Enable/disable is non-destructive and lossless: file-backed kinds
   toggle by rename; kinds embedded in shared config files toggle by a
   structured edit that preserves every unrelated key. Uninstalling the
   app changes nothing.
6. Never touch the unowned: unmanaged files are reported, never deleted;
   foreign symlinks are conflicts, not clobber targets; adoption merges
   content, never loses it.
7. Applies are transactional: preconditions revalidate against observed
   hashes immediately before mutation; pre-images are journaled first; any
   failure rolls back and interrupted applies recover on next launch.
   Removals go to a trash, never straight to delete.
8. One writer per scope: every apply (app or CLI) holds an OS-level scope
   lock; journal recovery runs under the same lock; a busy scope is a
   clear error, never an interleaved write.

## Decisions

- Tauri 2 · React 19 · Vite · Tailwind v4 · shadcn/ui · zustand ·
  tauri-specta · serde/toml.
- Vercel-inspired design language: monochrome, high contrast, minimal
  chrome. Light/dark/system only — no themes. Every color, space, and
  radius flows through design tokens (guard bans raw hex in UI code).
- No database: manifests, locks, and native dirs are the state; scans are
  in-memory views (startup, focus, watch); app prefs in one settings file.
- GUI + CLI are equal thin shells over `crates/core`; every core operation
  has a CLI verb. No CI until first release; `tools/guard` is the gate.
- Multi-harness kept (v1 fleet workflows depend on Pi). Every capability
  ships cross-harness through the capability table; a harness without
  native support for a kind is marked unsupported — never shimmed.
- Fresh manifest schema + one-time v1 importer; no compat shims. v1
  extras/theme packs are not carried over.

Build sequence and to-build specs: `docs/PLAN.md` — consumed and deleted as
phases complete; delete this pointer with that file.
