# HarnessKit adapter path extraction (pinned 461a7a1)

All paths use `~` for home. Citations are `file:line` within `crates/hk-core/src/`.

## claude (`adapter/claude.rs`)

| Item | Path(s) | Cite |
|---|---|---|
| base_dir | `~/.claude` | claude.rs:79-81 |
| detect() | `~/.claude` dir exists | claude.rs:83-85 |
| Skill dirs (global) | `~/.claude/skills` | claude.rs:87-89 |
| Skill dirs (project) | `.claude/skills` | claude.rs:284-286 |
| MCP config (global) | `~/.claude.json`, JSON key `mcpServers` | claude.rs:91-93, 112 |
| MCP config (project) | `.mcp.json` (same `mcpServers` key via `read_mcp_servers_from`) | claude.rs:288-290 |
| Hook config (global) | `~/.claude/settings.json`, key `hooks` (ClaudeLike format) | claude.rs:95-97, 156 |
| Hook config (project) | `.claude/settings.json` | claude.rs:292-295 |
| Plugin dirs | `~/.claude/plugins`; registry `~/.claude/plugins/installed_plugins.json`; marketplace map `~/.claude/plugins/known_marketplaces.json`; enabled state from `enabledPlugins` in `~/.claude/settings.json` | claude.rs:99-101, 297-330, 384-407 |
| Commands/workflows | `~/.claude/commands/*.md` + `~/.claude/output-styles/*.md` — surfaced under global **settings**, not workflow category | claude.rs:241-245 |
| Rules | global `~/.claude/CLAUDE.md` + `~/.claude/rules/**/*.md` (recursive); project `CLAUDE.md`, `.claude/CLAUDE.md`, `.claude/rules/**/*.md` | claude.rs:201-210, 260-266 |
| Settings patterns (project) | `.claude/settings.json`, `.claude/settings.local.json`, `.mcp.json` | claude.rs:268-274 |
| Subagents | global `~/.claude/agents/*.md`; project `.claude/agents/*.md` | claude.rs:248-251, 276-278 |
| External project memory | `~/.claude/projects/<encoded-cwd>/memory/*.md`, owner cwd read from first `*.jsonl` transcript's `"cwd"` field | claude.rs:212-232, 51-71 |
| Project markers | Dir `.claude`, File `.mcp.json` | claude.rs:253-258 |
| Remote MCP schema | `TypeAndUrl` (`{type: "http"\|"sse"\|"streamable-http", url, headers}`; bare `url` → Http) | claude.rs:140-142, mod.rs:181-189 |

## codex (`adapter/codex.rs`)

| Item | Path(s) | Cite |
|---|---|---|
| base_dir | `~/.codex` | codex.rs:117-119 |
| detect() | `~/.codex` dir exists | codex.rs:120-122 |
| Skill dirs (global) | `~/.agents/skills` (canonical, first), `~/.codex/skills` (deprecated) | codex.rs:123-133 |
| Skill dirs (project) | `.agents/skills` | codex.rs:223-227 |
| MCP config (global) | `~/.codex/config.toml`, TOML `[mcp_servers.<name>]` sections; remote = `url` + `http_headers`; `_hk_name` overrides key | codex.rs:134-139, 237, 248-282 |
| MCP config (project) | `.codex/config.toml`, same `[mcp_servers.*]` shape | codex.rs:201-208 |
| Hook config (global) | `~/.codex/hooks.json`, key `hooks` (ClaudeLike format) | codex.rs:140-142, 302-348 |
| Hook config (project) | `.codex/hooks.json` (inline `[hooks]` in config.toml NOT surfaced) | codex.rs:210-221 |
| Plugin dirs | `~/.codex/plugins`; cache `~/.codex/plugins/cache/{marketplace}/{plugin}/{version}/` with manifest `.codex-plugin/plugin.json` (latest semver wins); disabled via `[plugins."name@marketplace"] enabled = false` in `~/.codex/config.toml` | codex.rs:143-145, 350-444 |
| Commands/workflows | none (no `global_workflow_files` / `project_workflow_patterns` override) | — |
| Rules | global `~/.codex/<name>` and project `<name>` for each of `AGENTS.override.md`, `AGENTS.md`, `TEAM_GUIDE.md`, `.agents.md` — overridable via `project_doc_fallback_filenames` in `~/.codex/config.toml` | codex.rs:18-23, 79-101, 147-154, 189-191 |
| Memory (global) | `~/.codex/memories/*.md` (chronicle dir deliberately excluded) | codex.rs:169-183 |
| Settings | global `~/.codex/config.toml`, `~/.codex/hooks.json`; project `.codex/config.toml` | codex.rs:156-161, 193-195 |
| Subagents | global `~/.codex/agents/*.toml`; project `.codex/agents/*.toml` | codex.rs:163-167, 197-199 |
| Project markers | Dir `.codex` | codex.rs:185-187 |
| Remote MCP schema | `Toml` — HTTP only, no SSE | codex.rs:290-292, mod.rs:646-652 |

## cursor (`adapter/cursor.rs`)

| Item | Path(s) | Cite |
|---|---|---|
| base_dir | `~/.cursor` | cursor.rs:50-52 |
| detect() | `~/.cursor` dir exists | cursor.rs:54-56 |
| Skill dirs (global) | `~/.cursor/skills` (first), `~/.agents/skills` | cursor.rs:58-63 |
| Skill dirs (project) | `.cursor/skills` | cursor.rs:65-69 |
| MCP config (global) | `~/.cursor/mcp.json`, JSON key `mcpServers` | cursor.rs:71-73, 91 |
| MCP config (project) | `.cursor/mcp.json` | cursor.rs:204-206 |
| Hook config (global) | `~/.cursor/hooks.json`, key `hooks`, format `HookFormat::Cursor` (`{"command": "..."}`, no matcher) | cursor.rs:75-77, 43-45, 130-155 |
| Hook config (project) | `.cursor/hooks.json` | cursor.rs:208-212 |
| Plugin dirs | `~/.cursor/plugins`; scanned: `~/.cursor/plugins/local/{plugin}/.cursor-plugin/plugin.json` (source "local", always enabled) and `~/.cursor/plugins/cache/{marketplace}/{plugin}/` with optional `.cursor-plugin/plugin.json` | cursor.rs:79-81, 214-288 |
| Commands/workflows | none | — |
| Rules (project) | `.cursorrules`, `.cursor/rules/**/*.mdc` (plain .md ignored), `AGENTS.md` | cursor.rs:177-186 |
| Memory (project) | `.cursor/notepads/*.md` | cursor.rs:188-190 |
| Settings | global `~/.cursor/mcp.json`, `~/.cursor/permissions.json`, `~/.cursor/hooks.json`; project `.cursor/mcp.json` | cursor.rs:157-163, 192-194 |
| Subagents | global `~/.cursor/agents/*.md`; project `.cursor/agents/*.md` | cursor.rs:165-168, 196-198 |
| Ignore (project) | `.cursorignore`, `.cursorindexingignore` | cursor.rs:200-202 |
| Project markers | Dir `.cursor/rules`, File `.cursorrules` | cursor.rs:170-175 |
| Remote MCP schema | `PlainUrl` (`{url, headers}` → Http) | cursor.rs:118-120, mod.rs:195-203 |

## opencode (`adapter/opencode.rs`)

| Item | Path(s) | Cite |
|---|---|---|
| base_dir | `~/.config/opencode` | opencode.rs:153-155 |
| detect() | `~/.config/opencode` dir exists (deliberately not `which opencode`) | opencode.rs:157-164 |
| Skill dirs (global) | `~/.config/opencode/skills` (first), `~/.agents/skills` | opencode.rs:166-171 |
| Skill dirs (project) | `.opencode/skills` | opencode.rs:310-312 |
| MCP config (global) | `~/.config/opencode/opencode.jsonc` if it exists, else `opencode.json`; JSON key `mcp`; tagged union `{type:"local", command:[bin,...], environment}` / `{type:"remote", url, headers}`; per-entry `enabled` honored; parsed with jsonc-parser | opencode.rs:72-79, 173-182, 196-207, 84-133, 30-42 |
| MCP config (project) | `opencode.json` (relpath for capability flags); `mcp_config_path_for` override probes `<project>/opencode.jsonc` first | opencode.rs:314-335 |
| Hook config | same file as MCP (`hook_config_path` = `mcp_config_path`) but `HookFormat::None` and `read_hooks` returns `[]` — hooks are JS plugins | opencode.rs:137-139, 184-186, 209-211 |
| Plugin dirs | global `~/.config/opencode/plugins` (files `*.js/ts/mjs/cjs`, `.disabled` suffix = disabled, source "local"); project `.opencode/plugins` | opencode.rs:188-190, 213-238, 337-341, 56-66 |
| Commands/workflows | global `~/.config/opencode/commands/*.md` (workflow category); project `.opencode/commands/*.md` | opencode.rs:269-271, 298-302 |
| Rules | global `~/.config/opencode/AGENTS.md`; project `AGENTS.md` (CLAUDE.md fallback deliberately not claimed) | opencode.rs:240-242, 284-290 |
| Settings | global `opencode.json` + `opencode.jsonc` + `modes/*.md` + `themes/*.json`; project `opencode.json`, `opencode.jsonc` | opencode.rs:244-262, 292-296 |
| Subagents | global `~/.config/opencode/agents/*.md`; project `.opencode/agents/*.md` | opencode.rs:264-267, 304-308 |
| Project markers | Dir `.opencode`, File `opencode.json`, File `opencode.jsonc` | opencode.rs:273-282 |
| Remote MCP schema | `OpencodeRemote` (`{type:"remote", url, headers, enabled}` → read as Http) | opencode.rs:145-147, 112-118 |

## AgentCapabilities::from_adapter (mod.rs:624-654)

Pure derivation, no per-agent cases:
- `project_install.skill` = `!project_skill_dirs().is_empty()`
- `project_install.mcp` = `project_mcp_config_relpath().is_some()`
- `project_install.hook` = `project_hook_config_relpath().is_some()`
- `project_install.cli` = follows `skill` (CLI install deploys companion skill)
- `hooks_supported` = `hook_format() != HookFormat::None`
- `global_hook_install` = `supports_global_hook_install()` (false only for kiro)
- `mcp_remote.http` = `remote_mcp_schema() != Unsupported`; `mcp_remote.sse` = not `Unsupported` and not `Toml` (Codex is the sole HTTP-only agent)

Pinned matrix (mod.rs:823-857): claude T/T/T, codex T/T/T, cursor T/T/T, opencode skill T / mcp T / hook F (hooks_supported false — hooks are JS plugins).

## scanner.rs

- `scan_mcp_servers` (scanner.rs:250-324): delegates to `adapter.read_mcp_servers()` (each adapter owns its format — Claude/Cursor JSON `mcpServers`, Codex TOML `mcp_servers`, OpenCode JSON(C) `mcp`); timestamps from `mcp_config_path()` file ctime/mtime; names containing `/` (no space) become GitHub source URLs; permission profiles: remote = header keys as Env + host as Network (scanner.rs:329-364); stdio = env keys as Env, command basename as Shell, `npx`/`uvx` → Network `*`, absolute/`~/` args → FileSystem (scanner.rs:368-440); `enabled` passed through (only OpenCode/Hermes can be false).
- `scan_hooks` (scanner.rs:443-496): iterates `hook_config_paths_for(Global)` → `read_hooks_from`; extension name = `{event}:{matcher|*}:{command}`.
- `scan_plugins` (scanner.rs:499-586): delegates to `adapter.read_plugins()`; permissions inferred from plugin dir contents (`infer_plugin_permissions`); timestamps prefer registry values (Claude's `installedAt`/`lastUpdated`), else file times; source prefers `source_url` from agent manifest (Claude marketplace map), else `.git`-walk `detect_source`. Per-agent plugin formats: claude = `installed_plugins.json` registry (`plugins` object, keys `name@marketplace`) + `known_marketplaces.json` + settings `enabledPlugins`; codex = cache dir tree + `.codex-plugin/plugin.json` + config.toml `[plugins]` disable table; cursor = `local/` and `cache/` dirs + `.cursor-plugin/plugin.json`; opencode = flat JS/TS files, `.disabled` suffix.