# Claude Code

The first-class harness: every kind but Pi extensions has a native surface,
and the fleet's agent bodies and hook matchers are authored in Claude's own
vocabulary.

## Roots

| Scope | Path | Relocated by |
|---|---|---|
| Global | `~/.claude` | nothing — the adapter anchors it at `home` |
| Project | `<project>/.claude` | — |

Project markers: a `.claude/` directory, or a `.mcp.json` file at the repo
root. Owner: `crates/core/src/harness/claude.rs`.

## Surfaces

| Kind | Global | Project | Caps |
|---|---|---|---|
| agent | `~/.claude/agents/*.md` | `.claude/agents/*.md` | managed, both |
| skill | `~/.claude/skills/<name>/SKILL.md` | `.claude/skills/<name>/SKILL.md` | managed, both |
| command | `~/.claude/commands/*.md` | `.claude/commands/*.md` | managed, both |
| hook | `~/.claude/settings.json` → `hooks` | `.claude/settings.json` and `.claude/settings.local.json` → `hooks` | managed, both, **enforced** |
| mcp-server | `~/.claude.json` top-level `mcpServers` | `.mcp.json`, plus `~/.claude.json` `projects.<root>.mcpServers` | managed, both |
| plugin | `~/.claude/plugins/installed_plugins.json` joined with settings `enabledPlugins` | `.claude/settings.json` + `.claude/settings.local.json` `enabledPlugins` | observe + toggle, both |
| pi-extension | — | — | unsupported |

Plugin install and remove are parked with the marketplace work; only the
enable flip is written. MCP servers are written to `~/.claude.json` at global
scope and to the repository's `.mcp.json` at project scope
(`mcp_registry`, `crates/core/src/engine/targets.rs`).

## Format facts

- **Byte cap:** none. `format_caps` returns the defaults.
- **Name rule:** `Any` — any single path segment. Namespace separator `__`.
- **MCP transports:** stdio, streamable HTTP, SSE.
- **Agent file:** YAML frontmatter + markdown body, `<name>.md`. vstack
  writes `name`, `description`, `model`, `effort?`, `background`,
  `isolation?`, `memory?`, `tools` (allowlist, comma-joined), always
  `disallowedTools`, `color?`, `skills`, and a nested `hooks:` block for
  per-agent custom hooks (`crates/core/src/render/agent/claude.rs`).
- **Model dialect:** `fable` and `opus` resolve to the literal `inherit` —
  heavy tiers take the session model rather than pinning a possibly-smaller
  default. `sonnet` and `haiku` pin their own alias. Explicit vendor ids pass
  through (`crates/core/src/harness/models.rs`).
- **Tool vocabulary:** Claude's PascalCase names *are* the fleet's authoring
  vocabulary, so bodies pass through unrewritten and manifest tool names are
  only case-normalized — an unmapped lowercase name would silently fail to
  deny (`claude_tool_name`, `crates/core/src/render/vocab/mod.rs`).
- **Agent scoping:** per-agent file — hooks live in the agent's own
  `hooks:` block, so a scoped custom hook is enforced; every-agent custom
  hooks register in `settings.json`, covering the main session too.

## Hooks

Enforced: Claude runs the registered command and gates the tool call on its
exit status.

The script lands at `~/.claude/hooks/<name>.sh` or
`.claude/hooks/<name>.sh`, and the registration goes into that scope's
`settings.json` under `hooks.<event>` in the nested matcher-plus-handlers
shape. `settings.local.json` is observed and never written — it is the
user's own file. The registered command uses
`$CLAUDE_PROJECT_DIR` at project scope and an absolute path at global scope.
Timeouts travel in the seconds the source declares. Event names pass through
unmapped — the fleet's event vocabulary is Claude's.

Disabling renames the script to `<name>.sh.disabled` and reverses the
registration; nothing is lost either way
(`crates/core/src/engine/targets.rs`, `crates/core/src/engine/desired_kinds.rs`).

## Cross-reads — other tools read these files

GitHub Copilot CLI reads `.claude/settings.json` and
`.claude/settings.local.json` for a shared cross-tool subset —
`companyAnnouncements`, `disableAllHooks`, `enabledPlugins`,
`extraKnownMarketplaces`, `hooks` — and discovers skills from
`.claude/skills`; VS Code discovers agents from `.claude/agents`
([CLI configuration directory](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference),
accessed 2026-08-10).

Two consequences. A write vstack makes here changes Copilot's behavior, so
the blast radius of a Claude settings edit is wider than the file suggests —
invariant 2 covers the mechanics, not the reach. And the Copilot adapter
deliberately does not claim these paths: one file on disk would otherwise
count as two installations. The reach is reported as a note on the plan
(`cross_read_note`, `crates/core/src/engine/desired_skill.rs`).

## Skill placement

A project's shared skill tree is `.agents/skills/<name>`, and Claude's own
`.claude/skills/<name>` collapses onto it through a link when the bytes
match. Global variants live under the app data dir
(`rendered_skills_dir`, `crates/core/src/env.rs`).

## Validation

Rendered agents are checked before the plan is shown: frontmatter must exist
and must name the installed agent, or the plan refuses that harness
(`crates/core/src/render/validate/agent.rs`).
