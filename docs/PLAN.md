# Build Plan

Handoff document for fresh sessions. Read `AGENTS.md` and `docs/ARCHITECTURE.md`
first — they carry the rules and the system model; this file carries the work.
Consumable: delete items as they land; delete the file at the end of
Phase 6.

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

**Observation matrix (P1 pre-work, completed)** — read-only scan surfaces
for MCP servers, commands, and plugins. Ground truth: harness docs (Aug
2026) + HarnessKit adapters at the pinned SHA; details in
`docs/research/report-harnesskit-adapter-paths.md`. Mutation stays Phase
3; unsupported states are explicit; "observe-if-present" surfaces are
scanned when the path exists and never created.

| Harness | Commands (project / global) | MCP servers (project / global) | Plugins (project / global) |
|---|---|---|---|
| claude | `.claude/commands/*.md` / `~/.claude/commands/*.md` | `.mcp.json` key `mcpServers` / `~/.claude.json` key `mcpServers` (project-local servers also nest under `projects.<path>.mcpServers` in that file; toggles live in settings files: `enabledMcpjsonServers`, `disabledMcpServers`) | enablement: `enabledPlugins` in `.claude/settings.json` + `.claude/settings.local.json` (project) and `~/.claude/settings.json` (global); files: `~/.claude/plugins/` — `cache/`, `data/`, registries `installed_plugins.json` + `known_marketplaces.json` |
| codex | project: none; global `~/.codex/prompts/*.md` — deprecated-but-loading surface: observe, mark legacy, never mutate | `.codex/config.toml` / `~/.codex/config.toml` (root honors `$CODEX_HOME`), TOML table `[mcp_servers.<name>]` | none documented; observe-if-present `~/.codex/plugins/cache/…/.codex-plugin/plugin.json` + `[plugins."name@marketplace"]` enable table in config.toml |
| opencode | `.opencode/commands/*.md` (+ legacy singular `command/`) / `~/.config/opencode/commands/*.md` | `opencode.json`/`.jsonc` at repo root / `~/.config/opencode/opencode.json(c)` — key `mcp`, entries tagged `{type: local\|remote}`, per-entry `enabled` | `.opencode/plugins/*.{js,ts,mjs,cjs}` / `~/.config/opencode/plugins/` — `.disabled` filename suffix = disabled; npm refs in config `plugin` array |
| cursor | `.cursor/commands/*.md` / `~/.cursor/commands/*.md` | `.cursor/mcp.json` / `~/.cursor/mcp.json`, key `mcpServers` | none (VS Code-style editor extensions are out of scope); observe-if-present `~/.cursor/plugins/{local,cache}/…/.cursor-plugin/plugin.json` |
| pi | prompt templates `.pi/prompts/*.md` / `~/.pi/agent/prompts/*.md` | none — pi has no MCP surface | `packages[]` in `.pi/settings.json` / `~/.pi/agent/settings.json` (root honors `$PI_CODING_AGENT_DIR`): `npm:` entries, `./packages/<name>` relative entries (v1-installed), git refs; per-scope dirs `packages/`, `npm/`, `git/`, `extensions/*.{ts,js}` |

Cursor's global command/MCP surfaces exist and are observed even though v1
managed cursor project-only — mutation capability stays gated by the
capability table. Env overrides (`$CODEX_HOME`, `$OPENCODE_CONFIG`,
`$OPENCODE_CONFIG_DIR`, `$PI_CODING_AGENT_DIR`) apply wherever those roots
appear.

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

**Phase 3 declaration schema (pre-work, completed)** — source catalogs gain
`hooks/` (shell scripts with `# ---` comment frontmatter, v1 format),
`commands/<name>.md` (markdown body, claude-managed only), and
`mcp/<name>.toml` (fields: `command`, `args = []`, `transport =
"stdio"|"http"|"sse"`, `url`, `env = { KEY = "$VAR_REF" }` — env values
are always `$`-references, never secrets; validation rejects literals).
Manifest declaration tables: `[hooks.<name>]`, `[commands.<name>]`,
`[mcp-servers.<name>]` with the same `{source, harnesses?, method?,
enabled}` shape; `[plugins."<name@marketplace>"]` carries only `{enabled}`
plus marketplace provenance in the lock — plugins stay observe +
enable/disable until the marketplace work. `[pi-extensions.<name>]`
declares npm-packaged extensions from `pi-extensions/<name>/` in sources.
Mutation capability stays exactly the caps.rs table: hooks manage on
claude/codex (native JSON registration) + opencode (instruction render) +
cursor (project advisory .mdc); commands and MCP manage on claude only;
plugins toggle on claude only; pi-extensions manage on pi.

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
  local-source dir) and declares it from source `local`.
- **Per-kind flow (P2 pre-work, completed; v1 ground truth, mechanism
  unified)**:
  - **Skill** — source dir → rendered copy = tree copy + `[skill-instructions]`
    block injected into SKILL.md between `<!-- vstack:project-instructions -->`
    markers (author text never overwritten). Rendered canonical location:
    project `<proj>/.agents/skills/<name>`; global
    `~/.local/share/vstack2/rendered/skills/<name>`. Every harness-native
    skill dir gets a symlink (or copy, per `method`) → canonical; codex and
    pi read the project canonical natively (no link needed). v2 divergence
    from v1 mechanism, same outcome: v1 made codex's global dir the
    canonical; v2 keeps one rule — every native dir links to the rendered
    tree vstack owns. Never symlink into the source cache: refresh
    hard-resets it under the harness's feet.
  - **Agent** — always generated, never linked: per-harness render (claude
    yaml-md, codex toml, opencode md, cursor mdc, pi md) from the source
    agent file + manifest tables (`[agent-skills]`, launch/additional
    instructions, `[agent-frontmatter.<harness>]`, custom hooks), written
    straight into the native agents dir and overwritten on every refresh.
    `method` does not apply.
  - Lock `source_hash` covers source bytes + the manifest sections that
    shaped the artifact (invariant 3); hashing is SHA-256 over sorted
    relative-path+content pairs plus the serialized relevant sections.
  - P3 kinds follow the same split later: hooks/pi-extensions = script/pkg
    copy + structured registration edit; commands = file copy/symlink; MCP
    = structured config edit only; plugins = structured toggle only.
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

## User directives (2026-08-10, binding)

- License: MIT (LICENSE committed).
- UI work/validation/review: use lower-cost agent models where possible.
- A dedicated UI/UX polish phase is REQUIRED before release, validated by
  driving the real UI (agent-browser against a Chromium-drivable dev build
  with a `VITE_MOCK=1` in-memory command mock, since the WebKit Tauri
  window is not CDP-drivable): elegant, polished, consumer-friendly;
  simple, clean UX copy with no engineering jargon (rewrite: drift → "out
  of sync", unmanaged → "not managed yet", adopt → "start managing",
  orphaned → "left behind", scope → global/"per-project", harness →
  "tool"); progressive flows and onboarding (empty states each carry one
  primary action); clear action hierarchy (one primary action per view);
  well-considered surfaces for observability (Overview/Audit), repo
  modification (plan preview + confirm before any change), and Settings.

## In flight (delete as merged)

All 17 adversarial-review findings on the P2 engine are FIXED with
regression tests (crates/core/tests/review_fixes.rs). DONE since:
VITE_MOCK dev-mock bridge (`ui/src/dev/`), UI copy/IA/hierarchy rework
(plain-language vocabulary in `ui/src/lib/labels.ts`, confirm-dialog
plan previews), settings seeding in project applies, pi cross-scope
duplicate guard + fresh installs via update-pi, app-launch recovery,
report routing GUI (`report_route` + item-detail dialog), P5 end-to-end
over a file:// git host (`VSTACK_GIT_BASE`, cli/tests/remote_e2e.rs),
release workflow (.github/workflows/release.yml + docs/RELEASING.md,
icons, bundling; local deb/rpm packaging smoke green, AppImage needs
CI's FUSE), real v1 repo (drovr copy) migration smoke green.

Remaining, in order:

1. Browser-driven validation loop findings → fixes (agent run in flight).
2. GitHub repo bootstrap: create vanillagreencom/vstack2 (private until
   the user flips it), push, tag when ready; signing/updater keys are
   USER-supplied gates. In-place migration of real repos stays with the
   user (one tool per repo — v1 owns them until the binary swap).
3. Delete docs/research/ and this file.

## Phases (each ends with a working app)

1. **Walking skeleton** — DONE. Carried forward: fs-watch scan trigger and
   the Harnesses-page "open location" action land with Phase 2+ UI work.
2. **Declare & diff** — DONE. Kind-specific invariant-test extensions
   (structured-config toggles, shared-target ownership) land with Phase 3.
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

