## HarnessKit — Compressed Findings Report

Repo root: a clone of https://github.com/RealZST/HarnessKit at commit `461a7a1` (see PLAN.md References for re-clone instructions). All paths below are relative to that root unless noted.

### 1. What it is / tech stack / build

Cross-platform desktop+web+CLI manager for AI coding agent extensions (skills, MCP servers, plugins, hooks, agent-first CLIs) across 11 agents. Apache-2.0, v1.8.2 (`package.json:2-4`, `Cargo.toml:6`).

**Architecture: Tauri 2, not Electron.** Rust workspace (`Cargo.toml:1-2`, edition 2024) with 4 crates:
- `crates/hk-core` — all business logic (30,279 LOC total across its files per `wc -l`; biggest files: `store.rs` 3683, `deployer.rs` 3953, `scanner.rs` 3496, `service.rs` 3072, `manager.rs` 2892, `marketplace.rs` 1555, `kits/service.rs` 1304, `adapter/mod.rs` 1036).
- `crates/hk-desktop` — Tauri shell (`crates/hk-desktop/src/main.rs`, `tauri.conf.json`). macOS-only private API for window vibrancy/transparency (`tauri.conf.json:10,20-27`).
- `crates/hk-web` — Axum HTTP server that exposes the *same* command surface as REST, embeds the built frontend via `rust-embed` (`crates/hk-web/src/router.rs:18-20`), and is the backend for `hk serve` (web mode / headless/HPC use).
- `crates/hk-cli` — standalone `hk` binary (clap) with `status/list/info/audit/enable/disable/serve` subcommands (`crates/hk-cli/src/main.rs:20-107`).

Frontend: React 19 + TypeScript, Vite 6, TailwindCSS 4, Zustand 5 (state), React Router 7 (`HashRouter`), TanStack react-table v8, dnd-kit (drag/sort), react-i18next (en/zh/zh-TW, 10 namespaces per locale under `src/lib/i18n/locales/`), lucide-react icons, react-markdown (`package.json:14-46`).

Build: `npm run build` → `tsc && vite build` → `dist/`; `cargo tauri build` bundles it (macOS build script at `build.sh`); `install.sh`/`install.ps1` for the CLI. Biome for lint (`biome.json`), Vitest for frontend tests, `cargo test` for Rust (workspace has extensive `#[cfg(test)]` blocks embedded in nearly every source file — this is a well-tested codebase).

Data dir: `~/.harnesskit/` — SQLite `metadata.db` (`crates/hk-desktop/src/main.rs:14-19`, same path in CLI at `crates/hk-cli/src/main.rs:191-192`), plus `~/.harnesskit/web-token` for web-mode auth.

### 2. Full UI taxonomy

Routes defined in `src/App.tsx:189-202`, nav rendered in `src/components/layout/sidebar.tsx:19-30`.

**Sidebar order:** Overview → Agents → Extensions → Kits → Audit → Marketplace, then a separator, update-check card, **Scope Switcher**, then Settings.

- **Overview** (`src/pages/overview.tsx`) — dashboard/landing. Header shows a "terminal-style" agent/extension count readout, stat chips per extension kind, agent mascot cards. Sections: Tip of the Day (fetched from a GitHub-hosted JSON, cached in localStorage, `overview.tsx:41-63`), two-column Recent Activity (agent config file changes) / Recently Installed (skills/plugins/CLIs — MCP/hooks excluded because their timestamps are config-file-level not per-entry, `overview.tsx:372-378`) with deep-links into Agents/Extensions carrying scope in the URL, first-run onboarding cards, Quick Actions (View Agents / Run Audit / Check Updates / Marketplace).

- **Agents** (`src/pages/agents.tsx`) — two-pane: left `AgentList` (160px sidebar of all 11 agents), right `AgentDetail`. Per-agent dashboard shows config files grouped by category (Rules/Memory/Subagents/Settings/Workflow/Ignore — `ConfigCategory` enum, `crates/hk-core/src/models.rs:427-458`), with scope, path, size, live preview expansion, and an extension-count summary card. Custom user-added paths supported.

- **Extensions** (`src/pages/extensions.tsx`) — the main inventory table (`ExtensionTable`, TanStack react-table). Columns: name, kind, agent(s), permissions, audit/trust badge, status (`src/components/extensions/extension-table.tsx:52-167`). Filter bar (`ExtensionFilters`): kind/agent/pack/search. Batch mode (multi-select → enable/disable/toggle-by-pack). Check-updates / Update-all buttons. Row click opens a right-side detail drawer (`ExtensionDetail`, `src/components/extensions/extension-detail.tsx`) with: enable/disable toggle, editable source-pack binding (owner/repo chip), install metadata, per-scope location list, per-agent "Agents" chips, **Install to Agent** button row (gated by `AgentCapabilities` — scope/hook/MCP-transport support, `extension-detail.tsx:593-788`), Permissions breakdown (`PermissionDetail`), CLI-specific sections (parent/child linkage) via `CliSections`, per-agent Paths breakdown, file-tree + content preview (`SkillFileSection`), and a Delete flow with per-agent selection (`DeleteDialog`). "New skills discovered in tracked repos" banner exists in code but is currently feature-flagged off (`ENABLE_NEW_REPO_SKILLS = false`, `extensions.tsx:23`).

- **Kits** (`src/pages/kits.tsx`) — folder-grid of user-curated bundles ("Kits") that package skills/MCP/plugins/hooks + config files (rules/memory) into a portable `.hk-kit.zip`. Actions: New Kit (editor dialog with asset-candidate picker + config-file picker), Import (zip), multi-select → batch "Apply Selected" (installs to a project with conflict preview), per-kit detail drawer showing sync targets (which projects/agents have it installed, with `shared_with` warnings when two agents share a canonical dir), Export. Scope acts as a filter (global scope shows all Kits; project scope filters to only Kits installed there).

- **Marketplace** (`src/pages/marketplace.tsx`) — three tabs: Skill / CLI / MCP. Skills search skills.sh; MCP browses Smithery; CLI is a curated agent-first-CLI list. Detail panel per item: description, categories, install count/stars, third-party audit badges (ATH/Socket/Snyk risk + Socket numeric score), README/install-section extraction for CLI, SKILL.md preview for skills, per-agent "Install" buttons gated the same way as Extensions detail. "Install from Git URL" and "Install from local directory" dialogs live here too.

- **Audit** (`src/pages/audit.tsx`) — security scan results grouped per extension (deduped across agents), sorted worst-first by trust score. Search + trust-tier filter (Safe/Low Risk/Needs Review). Each row expands to show failed rules (with severity badge, per-rule description, all matching finding locations) and a "show all rules (incl. passed)" toggle. Deep-linkable via `?ext=`.

- **Settings** (`src/pages/settings.tsx`) — Agent Paths (per-agent enable toggle + editable/browsable config-root path, "All agents" vs "Detected only" visibility mode), Project Paths (add/remove/discover project roots, directory-picker or path-paste, auto-discovery when a non-project root is chosen), Appearance (theme: Tiesen/Claude; mode: system/light/dark; app icon — desktop only), Language (system/en/zh/zh-TW), update-check section (native Tauri updater on desktop, GitHub-release poll on web).

Additionally: **Scope Switcher** (sidebar) — global 3-way state: `all` / `global` / a specific registered `project` (`src/stores/scope-store.ts:8-11`) — filters Agents, Extensions, Kits, Audit simultaneously; persisted to localStorage and deep-linkable via `?scope=`.

### 3. Scanning / discovery mechanism

Core logic: `crates/hk-core/src/scanner.rs` (3496 lines) + `crates/hk-core/src/adapter/*.rs` (one file per agent: claude, codex, gemini, cursor, antigravity, copilot, windsurf, opencode, hermes, kiro, omp — `crates/hk-core/src/adapter/mod.rs:1-12`).

- **`AgentAdapter` trait** (`crates/hk-core/src/adapter/mod.rs:335-622`) is the single abstraction every agent implements: `detect()`, `skill_dirs()`, `mcp_config_path()`, `hook_config_path()`, `plugin_dirs()`, plus config-discovery methods for Rules/Memory/Subagents/Settings/Workflow/Ignore at both global and project scope (`project_*_patterns()` return relative globs like `"CLAUDE.md"`). `all_adapters()` returns the canonical ordered list of 11 (`adapter/mod.rs:670-684`); order must match `AGENT_ORDER` in `src/lib/types.ts`.
- **`AgentCapabilities::from_adapter`** (`adapter/mod.rs:624-655`) derives install-capability flags (project skill/mcp/hook install, global-hook support, MCP remote-transport support) purely from adapter declarations — this is the single source of truth the frontend gates install buttons against and the backend deploy code enforces, so they can't drift (test at `adapter/mod.rs:822-857`).
- **Scan functions** (`scanner.rs`): `scan_skill_dir` (parses `SKILL.md`/`SKILL.md.disabled`/standalone `.md`, frontmatter, `requires.bins`), `scan_mcp_servers`, `scan_hooks`, `scan_plugins`, `scan_cli_binaries` (uses `which`/`where` against a `KNOWN_CLIS` registry plus binaries referenced by skill frontmatter — `scanner.rs:17-136, 767-982`), `scan_project_extensions` (project-scoped variant), `scan_all` (orchestrates every adapter × every registered project). CLI parent/child linkage is back-filled after the full scan (`scanner.rs:1173-1218`).
- **Scope model**: `ConfigScope::Global | Project{name,path}` (`crates/hk-core/src/models.rs:477-495`). Extension IDs are deterministic FNV-1a hashes of `kind:agent:name` (global) or `kind:agent:name:project_path` (project) so re-scans are idempotent (`scanner.rs:76-136`).
- **Source attribution**: walks up from the resolved (symlink-followed) real path looking for `.git`; an authoritative `skills` CLI `.skill-lock.json` or an agent's own plugin-marketplace manifest overrides that heuristic when present (`from_manifest: true`, `scanner.rs:181-218`, `models.rs:90-99`).
- **Project discovery**: `discover_projects` recursively probes for any adapter's `project_markers()` (e.g. `.claude/` dir, `.mcp.json` file) up to a depth limit, skipping `node_modules`/`target`/etc. (`scanner.rs:1410-1491`).
- Trigger: `scan_and_sync` IPC command runs on app startup, on window-focus regain (desktop), and is debounced 5s (`src/App.tsx:27-137`).

### 4. Data model

**Core struct**: `Extension` (`crates/hk-core/src/models.rs:11-40`) — id, kind (Skill/Mcp/Plugin/Hook/Cli), name/description, `Source{origin: Git|Registry|Agent|Local, url, version, commit_hash, from_manifest}`, `agents: Vec<String>`, tags, `pack: Option<String>` (repo-group key), `permissions: Vec<Permission>` (tagged union: FileSystem/Network/Shell/Database/Env, each carrying its resource list — `models.rs:126-166`), enabled, `trust_score`, timestamps, `scope: ConfigScope`, `mcp_transport: Option<McpTransport>` (stdio/http/sse).

**Persistence**: SQLite via `rusqlite` (bundled), schema-versioned migrations `migrate_v1`..`v9` in `crates/hk-core/src/store.rs:184-457`. Key tables: `extensions` (denormalized JSON columns for source/agents/tags/permissions plus many install-meta scalar columns), `extension_agents` (join table for fast agent filtering, backfilled from JSON, v2), `audit_results`, `projects`, `hidden_extensions`, `agent_settings` (custom path + enabled + sort_order), `custom_config_paths`, and the Kits subsystem: `kits`, `kit_assets`, `kit_config_files`, `kit_sync_records` (v5, PK widened in v7 to allow multiple files per agent+category). DB is backed up automatically before every migration (`store.rs:197-200`).

**Kits** (`crates/hk-core/src/kits/types.rs`, `crates/hk-core/src/kits/service.rs` — 1304 lines): a Kit is an immutable zip snapshot (`.hk-kit.zip`) bundling `KitExtensionRef` (extension_id, asset_name, kind, content_hash, secrets-stripped flag) + `KitConfigFileRef` (agent/category/source file). `KitSyncRecord` tracks which project+agent pairs have synced a Kit, enabling conflict preview (`FileExists`/`DirExists` reasons) before overwrite. Service functions: `create_kit`, `update_kit`, `sync_kit_to_project`, `unsync_kit_from_project`, `preview_kit_project_conflicts`, `export_kit`/`import_kit` (`crates/hk-core/src/kits/service.rs:38-1154`).

**Config**: separate TOML file (not shown path but via `crates/hk-core/src/config.rs`) holds `GeneralConfig`(theme/update-check-hours), `AuditConfig` (per-rule enable flags, outdated-days threshold), `AgentPathOverrides` — distinct from the SQLite metadata store.

### 5. IPC / state architecture

Frontend never talks to two different backends with two different contracts — it's one command surface, two transports:

- `src/lib/transport.ts:24-33` — `transport(command, args)` detects Tauri (`__TAURI_INTERNALS__` on `window`) and calls `invoke()`; otherwise does `POST /api/{command}` with a snake_cased JSON body (`toSnakeKeys`, `transport.ts:68-75`, since Tauri's invoke auto-converts camelCase→snake_case but `fetch` doesn't).
- `src/lib/invoke.ts` defines the full `api.*` surface (~60 methods) — every method is a 1:1 wrapper around `transport("command_name", args)`. This is the entire frontend↔backend contract.
- **Desktop**: Tauri `#[tauri::command]` functions in `crates/hk-desktop/src/commands/*.rs`, registered in `main.rs`'s `invoke_handler!` macro (`crates/hk-desktop/src/main.rs:31-...`, list continues past what was read). `AppState` (store + adapters + pending_clones, all `Arc<Mutex<...>>`) is `.manage()`d once at startup.
- **Web**: `crates/hk-web/src/router.rs:72-164` — Axum routes, one per command, all `POST /api/{name}`, matching the same names 1:1 (e.g. `commands::list_kits` ↔ `/api/list_kits`). Handlers wrap blocking Rust work via `blocking()` (`router.rs:24-36`, spawns on tokio's blocking pool, mirroring how Tauri commands run off the async runtime). Frontend `dist/` is embedded via `rust-embed` and served as a SPA fallback (`router.rs:170-192`).
- **Auth (web mode only)**: optional bearer token, generated per `hk serve` invocation and persisted 0600 at `~/.harnesskit/web-token` unless `--no-token`/`--token` override (`crates/hk-cli/src/main.rs:195-286`); middleware `require_token` (`crates/hk-web/src/auth.rs`) only gates `/api/*`, and the frontend consumes a one-time `?token=` URL param à la Jupyter (`src/lib/transport.ts:58-65`), storing it in localStorage.
- **Frontend state**: Zustand stores per domain (`src/stores/{agent,extension,audit,kit,marketplace,project,scope,ui,toast,update,web-update}-store.ts`) — no Redux/Context; each store owns its own fetch/mutate methods that call `api.*`.
- **Push updates**: Tauri emits an `extensions-changed` event after background marketplace matching completes; frontend listens via `@tauri-apps/api/event` (`src/App.tsx:127-131`). Otherwise it's pull/poll (scan-on-focus, 5s debounce).

### 6. What "Audit" checks

18 static-analysis rules across `crates/hk-core/src/auditor/rules/{cli,content,mcp,permissions,plugin}.rs`, registered in `crates/hk-core/src/auditor/rules.rs:25-46`. Each implements the `AuditRule` trait (`id`, `severity`, `check(&AuditInput) -> Vec<AuditFinding>`, `crates/hk-core/src/auditor/mod.rs:30-34`).

Rules (severity/deduction from `src/pages/audit-utils.ts:5-157`, matches Rust `Severity::deduction()` in `models.rs:240-247`):
- **Critical (25pt)**: Prompt Injection, Remote Code Execution, Credential Theft, Plaintext Secrets, Safety Bypass.
- **High (15pt)**: Dangerous Commands, Broad Permissions, Permission Combination Risk, CLI Credential Storage, CLI Binary Source, MCP Command Injection.
- **Medium (8pt)**: Supply Chain Risk, CLI Network Access, CLI Permission Scope, CLI Aggregate Risk, Plugin Source Trust, Plugin Lifecycle Scripts.
- **Low (3pt)**: Unknown Source.

**Pre-processing**: content is deobfuscated first — strips zero-width spaces, directional-formatting Unicode, BOM, soft hyphen, variation selectors (`auditor/mod.rs:36-55`, explicitly credited as "inspired by AgentSeal's deobfuscation layer") so hidden-instruction tricks in Unicode can't dodge the text-pattern rules.

**Trust score**: starts at 100, subtracts each rule's deduction, but with same-rule dedup — first hit of a rule_id costs full deduction, every repeat hit of the *same* rule costs only 1 point (prevents e.g. 10 secret-leak lines from zeroing the score alone) — `auditor/mod.rs:104-117`. Tiers: Safe 80-100, Low Risk 60-79, Needs Review 0-59 (`models.rs:261-285`).

Scope: audits every extension instance independently per-agent (not deduped pre-audit) — rationale documented in README: "a safe copy on one agent doesn't guarantee safety on another" since files can drift between agent installs. Marketplace items also get pulled third-party audit data (ATH/Socket/Snyk risk + Socket numeric score) via `add-skill.vercel.sh` before install (`crates/hk-core/src/marketplace.rs:11-13`, `AuditSection` in `src/pages/marketplace.tsx:127-151`) — a second, independent pre-install signal distinct from HarnessKit's own post-install static rules.

### 7. Strengths to adopt / weaknesses to avoid

**Strengths:**
- **One capability-declaration source of truth.** `AgentAdapter` trait methods drive both what the deployer will actually write and what the UI greys out — tested explicitly so they can't drift (`adapter/mod.rs:624-655, 822-857`). Directly transferable pattern for a Rust+Tauri rebuild.
- **In-place, non-destructive management** (README "In-Place Management" section) — no shadow-copy/managed-folder model; enable/disable is a file rename; zero migration on uninstall. Strong trust/adoption story.
- **Deterministic, scope-aware stable IDs** (FNV1a hash including project path) make re-scans idempotent and let the same extension exist independently at global vs. per-project scope without collision.
- **One backend, two transports** — the entire IPC surface (`invoke.ts`) is transport-agnostic text commands; Tauri and a bundled Axum server both implement it 1:1, giving genuine desktop/headless-web parity from one codebase, not a second app. Good architecture reference if the rebuild wants an optional headless/server mode.
- **Trust-score dedup logic** (`compute_trust_score`) avoids the naive "10 lines flag the same rule = auto-zero" trap.
- **Grouping across agents** (`extensionGroupKey`) so the same logical skill installed under 3 agents shows as one row with per-agent sub-badges, not 3 duplicate rows — reduces list noise significantly.
- **Kits** is a genuinely novel differentiator vs. typical "extension manager" scope — portable curated bundles with conflict-preview-before-overwrite and cross-project sync tracking.
- **Security posture on the local web server** is well thought through: token file enforced even on `127.0.0.1` by default (loopback isn't isolated per-user on shared/HPC hosts), 0600 permissions checked and re-hardened, `?token=` URL stripped via `replaceState` to avoid Referer leakage (`hk-cli/src/main.rs:195-286`, `transport.ts:51-65`).
- **Deobfuscation pass before audit rules run** (Unicode trick stripping) is a nice, cheap catch that a naive regex-only audit engine would miss.

**Weaknesses / risks to avoid copying:**
- **hk-core is monolithic** — `store.rs` (3683 LOC), `deployer.rs` (3953), `scanner.rs` (3496), `service.rs` (3072), `manager.rs` (2892) are enormous single files mixing many concerns (DB access, JSON/TOML/YAML writers per agent, business logic) in one crate. A ground-up rebuild should split these earlier (e.g. per-adapter deploy modules, a dedicated persistence layer crate) rather than letting one crate absorb everything.
- **SQLite denormalization via JSON columns** (`source_json`, `agents_json`, `permissions_json` etc. in the `extensions` table, `store.rs:227-240`) works but pushes a lot of parse/serialize cost and makes SQL-level querying/filtering weak — the `extension_agents` join table (v2 migration) exists specifically to compensate for this. Consider proper relational columns/tables from the start instead of JSON blobs + later backfill migrations.
- **10 schema migrations already** for a v1.8 app, including a full-table-rebuild migration (v7, PK widening) and a drop-then-recreate (v8) — schema churn suggests the original data model wasn't fully thought through for Kits/scope before shipping. Worth spending more upfront design time on scope-aware and multi-target (Kit) modeling before writing the first migration.
- **Heuristic-heavy scanning**: CLI-binary detection walks `which`/`where` plus hardcoded fallback directories per OS (`scanner.rs:610-658`), and install-method detection is a lowercase substring match on path segments (`scanner.rs:680-703`) — fragile across unconventional install layouts (nvm, asdf, custom prefixes aren't covered).
- **Client-side re-implementation of backend logic**: trust-score computation, severity ordering/dedup are duplicated between Rust (`auditor/mod.rs`) and TypeScript (`audit-utils.ts:189-203`) with a comment noting they must be kept in sync manually — a rebuild should serialize this data instead of duplicating the algorithm in two languages.
- **Some UI complexity from ordering-fragile React effects**: multiple pages (`agents.tsx`, `extensions.tsx`, `audit.tsx`) have hand-documented effect-ordering constraints ("must be declared BEFORE the deep-link effect", "React 18 batches both updates and the router update gets dropped") to avoid state races between scope changes and deep-link handling — a sign the scope/deep-link/selection state model would benefit from a single coordinated state machine rather than several interacting `useEffect`s per page.
- **Feature flag left in shipped code** (`ENABLE_NEW_REPO_SKILLS = false`, `extensions.tsx:23`) — the backend discovery pipeline runs but the UI is dark; indicates the "surface new skills from tracked repos" feature wasn't fully baked before being wired end-to-end.
- **Third-party data dependencies**: marketplace (skills.sh, Smithery, add-skill.vercel.sh audit proxy) and Tip-of-the-Day content are fetched from specific hosted services with no visible self-host/offline fallback beyond a localStorage cache — a rebuild targeting enterprise/offline use should design these as pluggable/optional from day one.
- **Roadmap is thin** (`README.md:369-375` — "more agents," "more CLI commands") — no stated roadmap for e.g. plugin sandboxing, richer audit rule authoring, or team/multi-user features, suggesting the audit/security engine is the area most likely to need expansion for a "ground-up rebuild" to differentiate.

### Start Here (for the rebuild team)

Read `crates/hk-core/src/adapter/mod.rs` (trait + capability derivation) first — it's the single design decision that everything else (scanner, deployer, UI gating) depends on and is the cleanest piece of architecture to port directly. Pair it with `src/lib/invoke.ts` to see the exact command surface a Tauri+React clone would need to replicate.