# Build Plan

Handoff document for fresh sessions. Read `AGENTS.md` and `docs/ARCHITECTURE.md`
first — they carry the rules and the system model; this file carries the work.
Consumable: delete items as they land; delete the file at the end of
Phase 6.

**Step 0 (fresh clone or uncommitted repo): run `tools/setup` (installs the
git hooks), then make the baseline commit.**

## Context

vstack2 replaces vstack (v1): a desktop app plus a thin CLI with full parity
on core operations (consuming repos automate `refresh`/`report`; those flows
must never break) for managing AI coding-harness customizations. v1 is a Rust
CLI at `~/dev/vstack` (github.com/vanillagreencom/vstack) — it stays untouched
and running until Phase 6 ships. Develop locally only: no GitHub remote, no CI
until first release. v1 is the functional-parity reference, never a code
source. Preserve v1 *intent*; improve mechanisms freely when the outcome is
identical or better. One tool per repo: until a repo migrates (Phase 6), v1
keeps sole write ownership of it — v2's binary stays uninstalled (run via
`cargo run`) so it never shadows v1 on PATH.

## References

| Repo | Pinned | Take | Avoid |
|---|---|---|---|
| `~/dev/vstack` (v1) | `169eff98` | All semantics below; ground truth in `cli/src/{project_config,mapping,config,refresh_sources}.rs`, `cli/src/harness/mod.rs`, `README.md` | 4,766-line files; TUI |
| [RealZST/HarnessKit](https://github.com/RealZST/HarnessKit) | `461a7a1` | Adapter trait + capability table (`crates/hk-core/src/adapter/mod.rs`), page taxonomy, in-place management, audit rules engine | SQLite metadata store; 3-4k-line files; logic duplicated Rust↔TS; multi-theme system |
| [runkids/skillshare](https://github.com/runkids/skillshare) | `14959e2` | Adoption state machine, provenance-based conflict resolution, prune-only-managed, per-file SHA-256 metadata | Legacy-field config sprawl; state inside managed tree |
| [wshobson/agents](https://github.com/wshobson/agents) | `c4b82b0` | Validation errors carry a fix string; name-collision checks at edit time | Multi-harness content adapters (content-repo concern) |
| [fcakyon/claude-codex-settings](https://github.com/fcakyon/claude-codex-settings) | `eff6aca` | Canonical-manifest-with-sinks; symlink validation; cross-file consistency checks | Hand-synced manifest copies |

Deep-analysis reports for HarnessKit and skillshare are committed at
`docs/research/` (consumable — deleted with this file). Nothing else
persists outside this repo: to re-acquire a reference,
`git clone <url> && git -C <repo> checkout <pinned SHA>`.

## CLI compatibility contract (binding)

The binary is named `vstack` and is drop-in compatible for the surface
consuming repos automate: `refresh`, `report` (all v1 flags incl.
`--dry-run`), `verify` (non-zero exit on drift — composed in shell
pipelines), `list`, `check`, `add` — including v1's documented bare form
`vstack <source> [flags]`, which maps to `add` and is binding — `remove`,
`update-pi`, `update` (self-update), `init` (maintainer scaffolding). v1
flags, defaults, aliases (`ls`, `--pi-package`, …), and exit-code semantics
are the contract (ground truth: `cli/src/main.rs` clap definitions at the
pinned v1 commit); new capabilities get additive verbs (`adopt`,
`apply --plan`, `project add|discover`, `source add|enable|…`). `apply` is
**repurposed**, not additive: v1's `apply` installed theme-pack extras
(dropped in v2); its extras flags are non-binding.

**Sanctioned divergences** — Phase 6 compat tests consume this list instead
of failing on it: (1) skill removals are durable (v1 re-added them on
refresh); (2) `apply` repurposed as above.
Each verb's acceptance lands with its phase: `project` P1; `list`/`check`
P1→2; `add`/`remove`/`adopt`/`apply`/`refresh`/`verify` P2; `update-pi` P3;
`source` P5; `report`/`update`/`init` P6. Phase 6 runs fixture-based
black-box compatibility tests for **every** binding v1 verb before the
swap.

## Sources model (v1 intent, mechanism may improve)

A **source** is any git repo or local path with the v1 catalog shape
(`agents/ skills/ hooks/ pi-extensions/`, or a `[catalog]` table pointing
elsewhere). Outcomes to preserve:

- The default source — `vanillagreencom/vstack` on GitHub — is seeded only
  when a manifest is first created (from Phase 2 onward; remote *resolution*
  arrives Phase 5, until then it sits declared-pending without error), never
  re-added by later reconciliation: removing it is a durable choice. Any
  number of sources per scope.
- Removing a source is blocked while declarations still reference it (the
  UI offers disable instead); removal is allowed once nothing references it.
- Disabling a source deactivates (disables in place, per invariant 5) the
  installations declared from it; they stay declared, show as inactive —
  not drift — and re-enabling restores them.
- Every installed item records its source durably (resolved `owner/repo` or
  path); a recorded source is never silently rebound to another.
- Remote sources resolve through a local cache (v1: `~/.vstack/cache/…`,
  fetch + hard reset per resolve); refresh re-resolves and regenerates.
- The app must show, per scope (each vstack-enabled repo and global), which
  sources are enabled there and which items came from each — this visibility
  is a v2 requirement, not a v1 carryover.
- v2 improvement (allowed): declare sources explicitly in the per-scope
  manifest (a sources table with enable/disable) instead of v1's implicit
  per-item source strings; the importer derives it from v1 locks.

## Feature inventory → phase

| Capability | v1 | Phase |
|---|---|---|
| Scan/observe every kind, every harness, all scopes, read-only | partial | 1 |
| Project registration + discovery; durable layout | — | 1 |
| Manifest + lock + drift engine; adopt; apply w/ plan preview (skills, agents) | core of v1 | 2 |
| Transactional apply (journal, rollback, crash recovery) | — | 2 |
| Declared management of hooks, commands, MCP, plugins, pi-extensions | hooks, pi-ext only | 3 |
| Manifest customization editor (every project-manifest table) | toml-by-hand | 4 |
| Remote git sources, cache, refresh, per-scope source enablement | yes (no UI) | 5 |
| v1 importer; report routing; settings seeding; pi npm updates; release | yes | 6 |
| CLI verb for every core op — v1-compatible surface + new ops (adopt, apply --plan, project, source) | v1's whole surface | with each op's phase |
| Post-parity: kits/packs, registry browsing, marketplace install, security scoring | — | 7 (post-release) |
| Dropped: extras/theme packs, TUI | yes in v1 | never |

## v1 semantics to preserve (ground truth: `~/dev/vstack`)

**Manifest tables** (v2 keeps the concepts, fresh schema):

| Table | Semantics |
|---|---|
| `[agent-skills]` | Per-agent skill list. Once a project entry exists it is authoritative and auto prefix-matching is skipped; upstream additions merge in. **v2 divergence-by-design**: user removals become durable — v1's refresh code re-adds any removed skill the source still lists (its doc comments claim otherwise; a bug v2 fixes — see sanctioned divergences) |
| `[agent-launch-instructions]` / `[agent-additional-instructions]` | Injected into generated agents (top / bottom). Reserved key `all` (alias `*`) applies to every agent, renders first, wrapped in invisible comment markers so shared text is cleanly strippable on regeneration |
| `[skill-instructions]` | Same mechanism, injected into SKILL.md; applies to installed AND project-owned skills; never overwrites the skill author's text |
| `[agent-frontmatter.<harness>]` | Per-harness agent fields (model, color, effort, deny-tools, allowed-subagents, pane, isolation, memory, sandbox-mode, …). Source values are a floor; project values always win; missing keys are filled write-if-absent |
| `[[custom-hooks]]` | `{event, matcher?, command, description?, agents: "all"\|role\|[names]}` |
| `project-skills-dir` | Project-owned skills dir, symlinked into the harness skill dir so generated dirs stay untracked |
| Deny-lists only | Never write tool allowlists; harnesses keep native defaults minus `deny-tools` |

Source-side catalog tables (`[catalog]`, `[role-skills]`,
`[hook-events] "<Event>:<Matcher>" = all|[roles]`, source
`[agent-frontmatter.<harness>]` floors) are consumed by the engine when
resolving a source; they are edited in the catalog repo, not in this app.

**Lock entry**: `{name, kind, source, source_repo (durable owner/repo
provenance), harnesses[], method: symlink|copy, installed_at, source_hash}`.
Content hash covers source bytes PLUS the manifest sections that shape the
artifact (shared keys included) — editing a shared key invalidates
dependents. **v2 addition (not in v1's lock)**: persist the last-synced
upstream skill set per agent, making removal durability deterministic
across cache loss and machines; fixtures cover upstream-add, user-remove,
cache-loss, and second-machine cases.

**Other**: skills declare `dependencies: {required, optional}` in frontmatter,
auto-expanded on install. Agents carry a role (engineer / analyst / reviewer /
manager). Generated files are always overwritten on refresh; user intent
lives only in the manifest.

**Harness targets** (project / global) — v1 ground truth `cli/src/harness/mod.rs`:

| Harness | Agents | Skills | Hooks | Quirks |
|---|---|---|---|---|
| claude | `.claude/agents/` / `~/.claude/agents/` | `.claude/skills/` / `~/.claude/skills/` | `.claude/hooks/` + `settings.json` registration | full support |
| codex | `.codex/agents/*.toml` / `~/.codex/agents/` | `.agents/skills/` / `~/.codex/skills/` | `.codex/hooks/` + `hooks.json` + `[features] hooks=true` | agents are TOML; unsupported events (e.g. TaskCompleted) fall back to prose in developer_instructions |
| opencode | `.opencode/agents/` / config dir | `.opencode/skills/` | permission rule + `instructions/*.md` referenced from `opencode.json` | — |
| cursor | `.cursor/rules/*.mdc` | same dir | safety-advisory `.mdc` | project-only (no global scope), no agent frontmatter |
| pi | `.pi/agents/` / `~/.pi/agent/agents/` | `.agents/skills/` / `~/.pi/agent/skills/` | via pi-hooks extension | pi-extensions: npm packages; `pi.appendSystem` mirrored into `APPEND_SYSTEM.md` marker blocks; prod deps → `npm install --omit=dev`; also tracks `npm:` entries in Pi settings |

**New kinds** (cross-harness from day one; per-harness surfaces via the
capability table, unsupported kinds marked honestly): Claude targets are
commands `.claude/commands/*.md` / `~/.claude/commands/`; MCP servers project
`.mcp.json`, global `~/.claude.json` (`mcpServers`); plugins
`~/.claude/plugins/`. Plugins are observe + enable/disable ONLY through
Phase 3; adopt/install/remove are parked with the marketplace work. The
capability matrix is per-operation (observe, adopt, install, enable,
disable, remove, refresh), not just per-kind. Known: Codex MCP
lives in `~/.codex/config.toml` (`mcp_servers`); Codex has no slash-command
surface (unsupported).

**Phase 1 pre-work**: complete this matrix — every harness's *observation*
surfaces (read paths) for MCP servers, commands, and plugins — before
writing the scanner, and commit the result into this file. Derive from each
harness's docs plus HarnessKit's adapters
(`crates/hk-core/src/adapter/*.rs` at the pinned SHA), which implement
exactly this matrix. Mutation stays Phase 3; unsupported states must be
explicit from Phase 1.

## Manifest shape (canonical)

Fresh schema, one shape for both scopes (global manifest lives in the config
dir, project manifest in the repo). Finalized with schema validation +
fix-string errors in Phase 2; this example is the binding skeleton:

```toml
schema = 1

[sources.vstack]                  # sources declared per scope, by name
repo = "vanillagreencom/vstack"   # or: path = "../my-catalog"
enabled = true                    # seeded by default on fresh setup

[install]
harnesses = ["claude", "pi"]      # default targets for this scope
method = "symlink"                # or "copy"

[agents.orch]                     # declared items: [<kind-table>.<name>]
source = "vstack"                 # per-item overrides allowed:
[skills.github]                   #   harnesses = [...], method = "copy",
source = "vstack"                 #   enabled = false (default true)

# v1-carryover customization tables, semantics per the table above:
# [agent-skills], [agent-launch-instructions],
# [agent-additional-instructions], [skill-instructions],
# [agent-frontmatter.<harness>], [[custom-hooks]], project-skills-dir
```

Lock entries key by installation id (`kind:name:harness:scope`) and carry
the resolved provenance + content hash (see Lock entry above) plus the
applied enabled state — a declared-disabled item is disabled in place
(invariant 5), which is a state, not drift.

**Phase 3 pre-work**: schema fixtures for the new kinds before their engine
work — catalog layout (`commands/`, `mcp/` dirs in sources), declaration
fields (MCP: transport, args, env — secrets only as env *references*, never
values in the manifest; commands: markdown body; plugins: marketplace
provenance, observe/enable only), and the observation+mutation capability
matrix per harness.

## Durable layout (paths via the `dirs` crate per OS; Linux shown)

| What | Where |
|---|---|
| App settings: registered projects, path overrides, appearance | `~/.config/vstack2/settings.toml` |
| Global manifest + lock | `~/.config/vstack2/vstack.toml`, `~/.config/vstack2/lock.json` |
| Project manifest + lock | `<project>/vstack.toml`, `<project>/.vstack-lock.json` |
| Source cache | `~/.cache/vstack2/sources/<owner>_<repo>/` |
| Trash + apply journal + scope locks | `~/.local/share/vstack2/{trash,journal,locks}/` |
| Global local source (adopted content, global scope) | `~/.local/share/vstack2/local-source/` |

Discovery: the user registers roots; discovery walks a chosen root
(depth-limited; skips `node_modules`, `target`, `.git`) for harness markers
(`.claude/`, `vstack.toml`, `.mcp.json`, …). Paths are canonicalized and
deduplicated; a registered project that disappears is flagged in Scopes,
never auto-dropped.

## Engine approach (binding)

- Two identities. Logical **item**: `kind:name` within a source.
  **Installation**: item × harness × scope. Manifests declare items and
  their targets; locks, drift rows, and applies track installations; the
  Items page groups installations under their logical item.
- One `HarnessAdapter` trait: `detect()`, per-kind paths per scope, render.
  A static capability table derived from adapters — tested so UI gating and
  deploy behavior cannot drift (HarnessKit `adapter/mod.rs` pattern).
- Apply transaction: preflight every operation → journal pre-images →
  mutate → clear journal. Any failure rolls back from the journal;
  interrupted applies recover on next launch. Plans bind to the observed
  hashes they were computed from and revalidate immediately before each
  mutation — any mismatch aborts with a refreshed plan. Fault-injection
  tests cover every commit boundary (Phase 2).
- Content flow: source bytes → per-scope render (manifest customizations
  applied) → native target. Rendered artifacts live only in native dirs and
  are always overwritable. Adoption moves content into the scope's local
  source (project: `project-skills-dir` and siblings; global: the
  local-source dir) and declares it from source `local`. Phase 2 pre-work:
  spec the per-kind flow (symlink vs copy vs generated) with v1 as ground
  truth and commit it here.
- Physical target identity is modeled separately from installation identity
  — codex and pi share `.agents/skills/`. Scans dedupe shared artifacts,
  plans state coupled effects, and removal is reference-counted: an
  artifact disappears only when no harness references it. Fixtures cover
  shared-target cases.
- Scan triggers: startup, window focus, debounced fs-watch. In-memory only.
- Adoption state machine: target-has-files → merge into declaration;
  target-is-foreign-symlink → conflict, never clobber; broken → recreate.
- Validation findings always carry a machine-actionable fix; name collisions
  checked at edit/import time.

## UI spec

Vercel-inspired: monochrome, high contrast, minimal chrome. Light and dark
palettes only; "system" is an auto-selector between the two. Tokens only
(guard bans raw hex). shadcn/ui latest, components generated into
`ui/src/components/ui/`. One zustand store per domain over generated
bindings (`ui/src/bindings.ts` — the exact path the guard exempts and the
only file allowed to import `@tauri-apps`). No domain logic or recomputation
in TS.

| Page | Shows | Actions |
|---|---|---|
| Overview | harnesses detected, counts per kind, drift count, recent activity (session-only — no event store) | quick actions: scan, audit |
| Items | merged observed+declared table; filter kind/harness/scope/source/search | detail drawer: per-scope locations, provenance, enable/disable, deploy-to-harness (capability-gated), remove |
| Harnesses | per-harness dashboard: detected, version, config files w/ preview, counts | path override, open location |
| Scopes | global + each project; its enabled sources; what's installed where | register/discover project, toggle sources |
| Audit | drift rows: stale, missing, orphaned, unmanaged | adopt, reconcile, apply-plan preview |
| Sources | all sources; per-scope enablement; items per source; freshness | add/remove source, enable per scope, refresh |
| Settings | harness paths, projects, appearance | edit prefs (one settings file) |

Sidebar scope picker (Global / All / project) filters Items and Audit.

## Phases (each ends with a working app)

1. **Walking skeleton** — cargo workspace (core, app, cli) + ui; wire
   everything in "Wire at workspace creation" below. All five adapters
   (detect + paths), scanner reads **every kind** read-only across real
   global + registered projects; durable layout (settings file, project
   registry); Overview, Scopes, Items, Harnesses, and Settings pages render
   truth; CLI `project`, `list`, `check`. Adapter path fixtures for win/mac/linux
   land with the adapters. Phase 1 truth is deliberately reduced:
   observed/unmanaged items with best-effort git-origin provenance; no
   drift counts; source controls inert until Phase 5; `check` here is
   detection sanity, parity-grade in Phase 2. First vitest state tests land
   here and `vitest run` joins the guard.
   Done when: fixture-driven adapter/scanner tests + first UI tests pass
   deterministically, guard green; smoke: app + CLI show this machine's
   real installs.
2. **Declare & diff** — manifest, lock, drift engine, transactional apply,
   Audit page, adopt + apply with plan preview (skills, agents, local
   sources, all harnesses). Generic tests for all eight ARCHITECTURE
   invariants land as failing tests before the engine (kind-specific
   extensions — structured-config toggles, shared-target ownership —
   extend them in Phase 3+); fault-injection tests on the journal.
   CLI: add, remove, refresh, verify. v2 refuses to mutate a legacy
   (schema-less v1) manifest — hard "migration required" error until the
   Phase 6 importer; tests assert legacy files stay byte-identical.
   Done when: declare→apply→drift-clean round-trips on a fixture project
   (smoke: on a real one).
3. **All kinds** — declared management for hooks, commands, MCP servers,
   plugins, pi-extensions on every harness that supports them. CLI:
   update-pi.
   Done when: every kind manageable everywhere the capability table says
   it can be, and visibly gated where it cannot.
4. **Customization editor** — scope-aware GUI for every manifest table,
   identical workflows for global and project scope: agent×skill matrix,
   instruction editors (shared `all` key), per-harness frontmatter, custom
   hooks, project-skills-dir (project scope only).
   Done when: v1's toml-by-hand workflow is fully replaced in both scopes;
   every edit round-trips through apply + drift-clean.
5. **Remote sources & refresh** — git sources with cached clones, default
   source seeding, per-scope enablement UI, provenance conflict handling,
   refresh.
   Done when: a consuming repo installs from the default remote catalog,
   customizes, and refreshes clean — end to end, GUI and CLI.
6. **Migration & release** — v1 importer (manifest + lock + sources; skips
   extras; parses v1's deprecated `[agent-colors]` and drops it explicitly),
   tested against fixtures copied from real v1 files (`vstack.toml`,
   `.vstack-lock.json`); compat tests stub the networked verbs (`update`
   against a local release fixture, `report` via `--dry-run` routing plus a
   stubbed `gh`); report routing (GUI + CLI);
   settings seeding (`vstack.settings.toml` `[env]` merge); Pi npm-tracked
   updates. Release matrix: linux x86_64 (primary), macOS aarch64, windows
   x86_64; artifacts + update feed via GitHub Releases; packaging smoke per
   target. README, LICENSE, self-update (app + CLI). Release bootstrap:
   the GitHub remote and CI become allowed at this phase — create the repo
   and release workflows with native runners per target; signing and
   updater keys are user-supplied gates.
   Done when: the importer fixtures migrate green, and a real v1 repo's
   consuming flows (refresh, report) work unchanged as smoke. Delete
   `docs/research/` and this file.
7. **Parked (not a commitment)** — kits/packs, registry browsing,
   marketplace install, security scoring (severity deductions, repeat
   dedup, Unicode deobfuscation, block threshold). Revisit after release
   against real demand.

## Wire at workspace creation

- Root `Cargo.toml`:
  `[workspace.lints.rust] unsafe_code = "forbid"` and
  `[workspace.lints.clippy]` denying `unwrap_used`, `expect_used`,
  `dbg_macro`, `todo`, `unimplemented`, `print_stdout`, `print_stderr`,
  `too_many_lines`. **Every member crate needs `[lints] workspace = true`**
  (not inherited otherwise; guard enforces).
- `rust-toolchain.toml` pin; pin Node (`.nvmrc` or mise); commit
  `Cargo.lock` and `package-lock.json`; install via `npm ci` (guard already
  runs only lockfile-resolved tools via `npx --no-install`). Install
  Tauri's native OS prerequisites per its docs before first build.
- tsconfig `strict`; Biome denying `any` and `console.log`, with an
  override ignoring generated `ui/src/bindings.ts`.
- Bindings staleness check: a Rust test exports specta bindings and diffs
  them against the committed `ui/src/bindings.ts` (so `cargo test` in the
  guard catches drift).
- Non-interactive UI test command (`vitest run`) — first UI tests are a
  Phase 1 requirement, and the command joins the guard's UI block then.
- Upgrade the guard's core-purity check from TOML grep to `cargo tree`
  once the workspace exists.
- Widen the guard's hex-color check (3-digit hex, `rgb()`/`hsl()` literals)
  when UI work starts.
