# vstack v1 reference (repo `~/dev/vstack` @ 169eff98)

## 1. Per-harness paths & quirks

Harness ids: `claude-code` (alias `claude` in `from_id`), `cursor`, `opencode`, `codex`, `pi` — `cli/src/harness/mod.rs:53-72`. Cursor has NO global scope (`supports_global_scope`, mod.rs:74-84).

| Harness | Agents (project) | Agents (global) | Skills (project) | Skills (global) | Agent file |
|---|---|---|---|---|---|
| claude-code | `<proj>/.claude/agents/` | `~/.claude/agents/` | `<proj>/.claude/skills/` | `~/.claude/skills/` | `<name>.md` |
| cursor | `<proj>/.cursor/rules/` | — (unsupported) | `<proj>/.cursor/rules/` | — | `<name>.mdc` |
| opencode | `<proj>/.opencode/agents/` | `<oc_global>/agents/` | `<proj>/.opencode/skills/` | `<oc_global>/skills/` | `<name>.md` |
| codex | `<proj>/.codex/agents/` | `$CODEX_HOME/agents/` (default `~/.codex/agents/`) | `<proj>/.agents/skills/` | `$CODEX_HOME/skills/` | `<name>.toml` |
| pi | `<proj>/.pi/agents/` | `<pi_global>/agents/` | `<proj>/.agents/skills/` (**shared with codex**, single-rooted skill tree) | `<pi_global>/skills/` | `<name>.md` |

Sources: `agents_dir` mod.rs:87-131, `skills_dir` mod.rs:134-176, `agent_filename` mod.rs:284-289. Hooks dir exists only for claude-code: `.claude/hooks/` project, `~/.claude/hooks/` global (`hooks_dir` mod.rs:178-187).

Global-dir resolution (`cli/src/config.rs`):
- `claude_global_dir()` = `~/.claude` (config.rs:413)
- `cursor_global_dir()` = `~/.cursor` (config.rs:417)
- `opencode_global_dir()` = parent of `$OPENCODE_CONFIG`, else `$OPENCODE_CONFIG_DIR`, else `~/.config/opencode` (config.rs:421-430); `opencode_global_config_path()` = `$OPENCODE_CONFIG` or `<dir>/opencode.json` (config.rs:432); `opencode_project_config_path()` = `<proj>/opencode.json`, else `opencode.jsonc` if it exists, else `opencode.json` (config.rs:438-449)
- `codex_home_dir()` = `$CODEX_HOME` or `~/.codex` (config.rs:451)
- `pi_global_dir()` = `$PI_CODING_AGENT_DIR` or `~/.pi/agent` (config.rs:466); `pi_project_dir()` = `<proj>/.pi` (config.rs:478); `pi_settings_path(scope)` = `<scope>/settings.json` (config.rs:483); `pi_packages_dir` = `<scope>/packages` (config.rs:492); `pi_bin_dir` = `<scope>/bin` (config.rs:502); `pi_source_index_path` = `<scope>/.vstack-source.json` (config.rs:513)
- `project_root()`: walk up from CWD; `.vstack-lock.json` wins (even at `$HOME`); else first dir containing `.claude|.cursor|.codex|.opencode|.pi|.agents`, refusing `$HOME` itself (config.rs:523-575)
- Detection (`is_detected`, mod.rs:317-336): claude=`~/.claude` exists; cursor=`~/.cursor`; opencode=global dir OR global config OR project `opencode.json(c)`; codex=`$CODEX_HOME`/`~/.codex`; pi=global dir OR project `.pi/` OR `pi` binary on `$PATH` (mod.rs:339-351)

### claude-code agent generation (`cli/src/harness/claude.rs:11-104`)
YAML frontmatter: `name`, `description` (double-quoted+escaped), `model` (mapped via `agent.model_id("claude-code")`; opus→`inherit` per test claude.rs:456), optional `effort` (dropped for values ""|none|false|off|no, claude.rs:152-157), always `background:` (default = NOT effective pi-pane; pi pane defaults true for Engineer role or name=="planner" — claude.rs:117-136), optional `isolation`, `memory`, `disallowedTools: A, B` (defaults: `Agent` always; `AskUserQuestion` unless name=="planner"; plus mapped `deny-tools` — claude.rs:159-202 with alias normalization table), `color` (frontmatter > extras > agent), `skills: a, b` (comma list), inline `hooks:` YAML (event→matcher→`{type: command, command}`; installed hooks render as `bash "$CLAUDE_PROJECT_DIR/.claude/hooks/<name>.sh"` — claude.rs:216-258). `tools:` overrides are deliberately ignored (deny-only model). Body: do-not-edit blockquote, guidance+skills preamble inserted after intro, custom-hook prose + additional instructions appended (claude.rs:83-96).

### codex quirks (`cli/src/harness/codex.rs`)
- Agents are **TOML**: `name`, `nickname_candidates = ["Name-Atlas",…]` (default suffixes Atlas/Delta/Echo/Nova/Orion/Vector, `tpm`→`TPM`; codex.rs:145-182), `description`, `model` (opus/sonnet/haiku → `gpt-5.6-sol`; codex.rs:40-46,193-198), optional `model_reasoning_effort`, `sandbox_mode` (role default: Analyst/Reviewer/Manager→`workspace-write`, Engineer→`danger-full-access`; codex.rs:31-38), then `developer_instructions = '''…'''` containing the whole body (codex.rs:85-101).
- No native `skills` field: emits a `## Required Skills` prose section with per-skill path `.agents/skills/<name>/SKILL.md` (project) or `$CODEX_HOME/skills/<name>/SKILL.md` (global) (codex.rs:107-139).
- Hooks NOT in agent file (codex.rs:11-15) — see §2.

### cursor quirks (`cli/src/harness/cursor.rs:10-47`)
`.cursor/rules/<name>.mdc`, frontmatter only `description: "<name> — <desc>"` + `alwaysApply: false`. Skills/hooks params ignored. Hook install writes separate `.cursor/rules/safety-<hook>.mdc` with `alwaysApply: true` + safety prose (installer/hooks.rs:250-262,684-699).

### opencode quirks (`cli/src/harness/opencode.rs:11-75`)
Frontmatter: `description`, `mode` (default `subagent`; override `all`→`subagent`; opencode.rs:99-105), `model` = `agent.model_id("openai")` mapping, `color` (names mapped to hex, e.g. green→`#22c55e`, or verbatim `#rrggbb`; opencode.rs:127-151), `options:{reasoningEffort, reasoningSummary: auto, textVerbosity: medium}` only when effort set, `permission:` map with `<perm>: deny` — defaults (subagent mode only): `task: deny` + `question: deny` (unless planner); deny-tools mapped via `opencode_permission_name` (write/patch/multiedit→`edit`, glob/find/ls→`glob`, webfetch/websearch/etc→`webfetch`; opencode.rs:153-198).

### pi quirks (`cli/src/harness/pi.rs:26-94`)
Frontmatter: `name`, `description` (quoted), `deny-tools:` comma list — defaults `subagent, get_subagent_result, steer_subagent, stop_subagent` (+`delegate_subagent` when allowlist empty; +`question` unless planner; +`tasks_write` for Reviewer role; pi.rs:192-212); `allowed-subagents:` comma list — default `["scout"]` for Engineer role only, explicit empty list disables (pi.rs:214-237); non-empty allowlist strips the default `delegate_subagent` deny unless user explicitly denied it (dash/underscore-normalized; pi.rs:165-190); `model:` — opus omits model entirely (inherit parent session); sonnet/haiku → `openai-codex/gpt-5.6-sol[:effort]`; `inherit|current|parent` override omits; other strings verbatim with optional `:effort` suffix (pi.rs:96-148); `color`; `pane: true` when Engineer role or name=="planner" (or override) (pi.rs:68-73).

### pi packages (npm) (`cli/src/pi_extension.rs`)
- Source: `<source>/pi-extensions/<name>/package.json`; parsed fields `name, description, version, keywords, pi.extensions[], pi.appendSystem, bin` (string or map) (pi_extension.rs:146-247).
- Install flow (pi_extension.rs:484-553): (1) remove same-scope legacy renamed packages (rename table `PI_EXTENSION_RENAMES` — 1.0.0 moved to `@vanillagreen/` npm scope; pi_extension.rs:278+); (2) skip if same/legacy name installed at the OTHER scope (Pi loads both scopes, duplicates crash startup); (3) copy dir to `<scope>/packages/<name>` (skips `node_modules,.git,.turbo,.next,.cache,build,out,coverage,.pi,.test-output`); (4) if package.json has non-empty `dependencies`/`optionalDependencies`, run `npm install --omit=dev --package-lock=false --legacy-peer-deps --no-audit --no-fund` in dest, hard error with recovery command on failure (pi_extension.rs:884-971); (5) symlink each `bin` entry at `<scope>/bin/<cli>` (pi_extension.rs:695-718); (6) add relative entry `"./packages/<name>"` to `settings.json` `packages[]` — replaces existing match **in place** (preserving load order); dedupe recognizes legacy absolute paths and `{"source": …}` objects (pi_extension.rs:720-793); (7) upsert appendSystem block; (8) update `.vstack-source.json` source index (tracks source repo + git HEAD for update detection).
- `APPEND_SYSTEM.md`: `<pi_global>/APPEND_SYSTEM.md` (global) / `<proj>/.pi/APPEND_SYSTEM.md` (project) (pi_extension.rs:978-984). Marker blocks: `<!-- vstack:append-system <name> begin -->` … `<!-- vstack:append-system <name> end -->` (pi_extension.rs:986-991); upsert strips existing block then appends `stripped + "\n\n" + block + "\n"` (pi_extension.rs:1040-1069); unterminated markers are left untouched; remove can delete the file when empty.
- `settings.json` `npm:` entries: `list_npm_packages` reads `packages[]` string entries with prefix `npm:` (`npm:<name>` / `npm:<name>@<ver>` / scoped `npm:@scope/pkg@ver`), returns bare names (pi_extension.rs:59-97). `list_installed_vstack_packages` scans `<scope>/packages/*/package.json`, recursing one level into `@scope/` dirs (pi_extension.rs:107-144).
- Pi per-hook install is a **no-op** (`Harness::Pi => {}`, installer/hooks.rs:153,740) — the native `pi-hooks` extension owns behavior.

## 2. Hook registration per harness (`cli/src/installer/hooks.rs`)

Hook source format: shell script with YAML-in-comments frontmatter between `# ---` lines; fields `name, event, matcher, description, safety, timeout, harnesses` (harnesses = allowlist of harness ids, string CSV or list, None=all) (`cli/src/hook.rs:5-201`).

- **claude-code** (hooks.rs:165-248): write script to `<scope>/.claude/hooks/<name>.sh` (chmod 755); merge into `<scope>/settings.json` (`~/.claude/settings.json` global) structure: `{"hooks": {"<Event>": [{"matcher": "...", "hooks": [{"type":"command","command":"...","timeout":N}]}]}}`. `timeout` sits on the handler object, not the matcher group. Command = `bash "$CLAUDE_PROJECT_DIR/.claude/hooks/<name>.sh"` (project) or `bash <abs path>` (global). Rerun-idempotent: entries matching owned commands are removed before re-adding (hooks.rs:64-104).
- **codex** (hooks.rs:306-439): if `codex_event_for(event)` maps (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PreCompact, PostCompact, PermissionRequest, Stop → identity; `TaskCompleted` → None, hooks.rs:280-292): native install = script at `<root>/hooks/<name>.sh` (root = `$CODEX_HOME` or `<proj>/.codex`), merge into `<root>/hooks.json` (same JSON shape as claude settings' `hooks` object, wrapped in `{"hooks": {...}}`), command = `bash "$(git rev-parse --show-toplevel)/.codex/hooks/<name>.sh"` (project) or `bash <abs>` (global), then ensure `[features]\nhooks = true` in `<root>/config.toml` via a text-level merge preserving comments/ordering and migrating deprecated `codex_hooks` key (hooks.rs:445-634). Unmapped events fall back to prose: append `## Safety: <name>` block inside each agent TOML's `developer_instructions` before closing `'''` (hooks.rs:639-669). Removal strips hooks.json entry (deleting file if emptied), strips prose blocks, leaves `hooks = true` (hooks.rs:721-731,779-861).
- **opencode** (installer/hooks/opencode.rs:9-117): no hooks — converts to (a) instruction file `<scope>/instructions/vstack-hook-<name>.md` (`.opencode/instructions/` project) containing `# Safety: <name>` + safety prose; (b) `opencode.json` edits: append `"instructions/vstack-hook-<name>.md"` (global ref) or `".opencode/instructions/vstack-hook-<name>.md"` (project ref) to `instructions[]`; and for `PreToolUse`+matcher `Bash` only, set `permission.bash = {"*": "ask"}` if absent. New config gets `{"$schema": "https://opencode.ai/config.json"}`. Removal drops the instruction ref (plus legacy inline prose by keyword match), deletes the file, and removes the `bash:{"*":"ask"}` permission only when no `vstack-hook-` instructions remain (opencode.rs:120-231).
- **cursor**: `.cursor/rules/safety-<name>.mdc`, `alwaysApply: true` (hooks.rs:691-699).
- **pi**: nothing.

Custom hooks (from project config) render into claude agent frontmatter alongside installed hooks (command verbatim) (claude.rs:233-241) and as prose sections in other harnesses (`agent::custom_hooks_section`).

## 3. refresh_sources.rs — cache & resolution

- Remote cache: `~/.vstack/cache/<key>` where `key = source.replace('/', "_")` (`global_base_dir()` = `$HOME`; refresh_sources.rs:309-316). A dir counts as cached only if `<cached>/.git` exists.
- Update flow (`update_cached_repo`, refresh_sources.rs:371-391): `git fetch origin --quiet` then `git reset --hard origin/HEAD`; failures degrade to warnings ("using cached version"). Same flow in `config::refresh_remote_caches` (config.rs:819-851) which iterates unique remote (owner/repo-shaped) lock sources.
- Source string classes (refresh_sources.rs:271-356): absolute path (must be dir; discovery additionally requires `is_vstack_source` layout heuristic); explicit relative `.`/`./…`/`../…` (resolved against `project_root()`, canonicalized); bare token (legacy: project-relative then walk-up-from-CWD, non-discovery only); remote shorthand = contains `/`, not starting with `.` or `/` → cache lookup. `~`-prefixed never resolves relative.
- `resolve_recorded_source` (refresh_sources.rs:244-253) skips the layout heuristic — a lock entry's recorded source accepts any directory (fixes silently dropped dot-named / skills-only sources); `resolve_single_source` (discovery, refresh_sources.rs:230) applies heuristic + updates remote.
- `resolve_source_records` (refresh_sources.rs:61-110): resolve each unique lock-entry source once; fallbacks when empty: walk up from CWD for a vstack source; then registry (`sources.json`) `current` + `entries`. Dedupe by canonicalized root, accumulating aliases; `source_repo` (`owner/repo`) attributed from remote shorthand or git origin, never inferred from local layout (config.rs:947).
- `refresh_source_for_entry` (refresh_sources.rs:145-176): match by alias, then absolute-path identity; single-source fallback ONLY if the entry's recorded source no longer exists on disk.
- `RefreshSource` (refresh_sources.rs:17-40) = root + aliases + source_repo + `MappingConfig` + discovered agents/skills/hooks/pi_extensions.

## 4. project_config.rs / config.rs — manifest & lock

Config file selection (`project_config_path`, project_config.rs:15-35): `<root>/vstack.toml`, unless it has top-level `is_source_catalog = true` → writes go to `<root>/vstack-local.toml`.

`ProjectConfig` tables (project_config.rs:46-86):
- `[agent-skills]` — `HashMap<String, Vec<String>>`, agent→skills; authoritative for generated frontmatter; presence of a key disables prefix matching for that agent.
- `[agent-colors]` — agent→color string; empty values ignored, fall back to `[agent-frontmatter].color` (`color_for`, project_config.rs:283-297).
- `[agent-launch-instructions]` (alias `agent-guidance`) — agent→string; rendered as `## Launch Instructions`.
- `[agent-additional-instructions]` (alias `agent-instructions`) — agent→string; rendered as `## Additional Instructions` (appended at body end).
- `[skill-instructions]` — skill→string; injected into installed SKILL.md as `## Project Instructions` between `<!-- vstack:project-instructions:start/end -->` markers right after frontmatter (skill.rs:61-163).
- Shared key: `all` (alias `"*"`) in all three instruction tables applies fleet-wide (project_config.rs:107-127); rendered shared text is wrapped in `<!-- vstack:shared-instructions:start/end -->` markers, shared first + blank line + specific (merge/strip logic project_config.rs:355-449).
- `project-skills-dir` — optional string; refresh links each `<dir>/<name>` into `.agents/skills/<name>` to keep tracked skills out of `.agents` (project_config.rs:72-82).
- `[[custom-hooks]]` — array of `{event, matcher?, command, description?, agents}`; `agents` default `"all"`, or list matching agent names OR role strings (`custom_hooks_for`, project_config.rs:89-104,451-474).
- `[agent-frontmatter]` (legacy, harness-agnostic) and `[agent-frontmatter.<harness>.<agent>]` — parsed manually (`parse_agent_frontmatter_tables`, project_config.rs:162-206); a table is treated as an override iff it contains any known override key (project_config.rs:131-160). Typed fields (`AgentFrontmatterOverrides`, agent.rs:229-283, kebab-case): `color`, `model`, `tools` (legacy, ignored), `deny-tools`, `allowed-subagents` (aliases `allowedSubagents`, `subagent-agents`, `subagent_agents`), `pane`, `background`, `effort`, `isolation`, `memory`, `mode`, `sandbox-mode`, `model-reasoning-effort`, `nickname-candidates` (aliases camel/snake). List fields accept CSV string or list. Source `vstack.toml` may define the same tables as defaults; project entries win per-field, except `deny_tools` which MERGES both lists (`merge`, agent.rs:311+; `overlay_source_frontmatter`, project_config.rs:258-276). Normal generation reads only the per-harness table (`frontmatter_for`, project_config.rs:299-318); legacy table returned only for empty harness id.

Lock file (`config.rs:8-107`): `.vstack-lock.json` at project root; global at `~/.config/vstack/.vstack-lock.json` (`lock_file_path`, config.rs:363-369). Shape: `{"version": u32, "entries": {"<name>": {…}}}` (BTreeMap). `LockEntry` fields: `name`, `kind` (kebab: `skill|agent|hook|pi-extension|extra`), `source` (string as recorded), `source_repo` (optional `owner/repo`, skipped if None), `harnesses` (list of ids), `method` (`symlink|copy`), `installed_at` (ISO8601 string), `source_hash` (FNV-1a content hash string, skipped if empty; hashing includes relevant vstack.toml sections — `AGENT_SHARED_TABLES`/`SKILL_SHARED_TABLES`, config.rs:780-786). JSON always newline-terminated (`to_json_pretty`, config.rs:128).

Global config files: `global_state_dir()` = `~/.config/vstack/` holding `.vstack-lock.json`, `sources.json` (`SourceRegistry`: `current`, `entries[]`, `removed_entries[]`, `project_current{project_key→source}`, config.rs:109-126), and `skills/<name>` (global canonical skill copies for non-codex harnesses; installer.rs:77-89). Codex global canonical = `$CODEX_HOME/skills/<name>`; project canonical = `<proj>/.agents/skills/<name>` with `.vstack-refreshed` PID marker; symlink method links harness dirs → canonical (installer.rs:66-177).

## 5. mapping.rs — source-side `vstack.toml`

`MappingConfig::load(source_dir)` reads `<source>/vstack.toml` (mapping.rs:62-75). Tables:
- `[catalog]`: `agents|skills|hooks|pi-extensions|extras` = optional path lists, defaulting per `CatalogKind::default_paths()` (mapping.rs:27-52).
- `[agent-skills]` agent→skills (same semantics as project; explicit entry suppresses prefix matching; `reviewer-<x>` falls back to key `<x>`; mapping.rs:77-133). Matching order: prefix matches (`prefixed_skill_matches`) unless explicit → explicit agent-skills → reviewer-stripped → `[role-skills]`; filtered to available; sorted.
- `[role-skills]` role(`reviewer|engineer|analyst|manager`)→skills.
- `[hook-events]`: key `"<Event>:<Matcher>"` (or `"<Event>:"` for event-only) → value `"all"` or `["engineer", …]` (untagged `HookTarget`, mapping.rs:54-59). `hooks_for_agent` tries exact key then event-only; empty table falls back to `agent::match_hooks` heuristic (mapping.rs:135-165).
- `[agent-frontmatter]`/`[agent-frontmatter.<harness>]` parsed via the shared project_config parser as defaults beneath project overrides (mapping.rs:15-23,70-74).

## 6. Skill frontmatter & dependency expansion (`cli/src/skill.rs`)

SKILL.md YAML frontmatter: `name`, `description`, `license?`, `user-invocable?` (bool), `dependencies: {required: [names], optional: [names]}` (skill.rs:7-36). If `dependencies` absent, a body fallback parser scans a `## (Skill )Dependencies` markdown table for backtick names / "Xxx skill" patterns, `(optional)` marking optional, ignoring reverse-dependency tables (skill.rs:289-419). `build_dependency_graph` keeps only required deps that exist in the discovered skill set (skill.rs:243-261). `expand_dependencies(selected, graph)` = BFS transitive closure over required deps, returns (expanded, auto-added) (skill.rs:265-287). Discovery = `<dir>/<sub>/SKILL.md`, sorted by name (skill.rs:220-240).