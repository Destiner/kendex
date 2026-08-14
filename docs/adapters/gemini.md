# Gemini CLI

The most symmetric adapter after Claude: both scopes hold the same layout
under their own root, so the surface lists differ only in where they start.
Two things complicate it — a system settings layer that outranks project
scope, and one machine-wide file recording whether each MCP server is on.

Facts below were verified against Gemini CLI's own docs on `main`, accessed
2026-08-10 (roots, precedence, kinds) and 2026-08-13 (hook events, timeout
units, settings categories).

## Roots

| Scope | Path | Relocated by |
|---|---|---|
| Global | `~/.gemini` | nothing — no documented variable relocates the root itself |
| Project | `<project>/.gemini` | — |

Project markers: a `.gemini/` directory, or a `GEMINI.md` file at the repo
root. `gemini-extension.json` is *not* a marker — it marks a repository that
publishes an extension, not one that uses the CLI. Owner:
`crates/core/src/harness/gemini/mod.rs`.

## Surfaces

| Kind | Global | Project | Caps |
|---|---|---|---|
| agent | `~/.gemini/agents/*.md` | `.gemini/agents/*.md` | managed, both |
| skill | `~/.gemini/skills/<name>/SKILL.md` | `.gemini/skills/<name>/SKILL.md` | managed, both |
| command | `~/.gemini/commands/**/*.toml` | `.gemini/commands/**/*.toml` | managed, both |
| hook | `~/.gemini/settings.json` → `hooks` | `.gemini/settings.json` → `hooks` | managed, both, **enforced** |
| mcp-server | `~/.gemini/settings.json` → `mcpServers` | `.gemini/settings.json` → `mcpServers` | install/remove/refresh both, **toggle global only** |
| plugin (extension) | `~/.gemini/extensions/<name>/gemini-extension.json` | — none; there is no project extension directory | observe only, global |
| pi-extension | — | — | unsupported |

Extensions install globally only and their enablement is an undocumented
path-rule file (`extension-enablement.json`, `!` prefix to disable, trailing
`*` to include subdirectories), so nothing there is ever written.

An MCP server is *declared* per scope but the file recording whether it is
switched on is a single global one. Switching one off is therefore a global
act; doing it under a project lock would write outside the scope holding the
lock, so the toggle exists only at global scope and a project-scope disable is
declined with a note saying to remove the declaration instead.

## Format facts

- **Byte cap:** none.
- **Name rule:** `Any`. Namespace separator `__`.
- **MCP transports:** stdio, streamable HTTP, SSE — but the *keys* differ
  from every other tool. A command server keeps `command`; a streamable-HTTP
  endpoint is `httpUrl`; an SSE one is plain `url`; there is no `type` beside
  either, and vstack strips one if the source wrote it. Written in another
  tool's shape, an HTTP server would load as SSE and reach nothing
  (`server`, `crates/core/src/engine/gemini.rs`).
- **Agent file:** YAML frontmatter + markdown body, where the body is the
  system prompt. vstack writes `name`, `description`, `kind: local`, `model`
  and `tools`. `kind: local` is explicit because it is the only kind vstack
  manages — a remote subagent runs off this machine. Skills and per-agent
  hooks are not frontmatter fields, so both travel as prose inside the system
  prompt (`crates/core/src/render/agent/gemini.rs`).
- **Model dialect:** `fable` and `opus` resolve to `gemini-3-pro-preview`,
  `sonnet` and `haiku` to `gemini-3-flash-preview`; `inherit` is spelled
  literally, in agent frontmatter only. The 2.5 GA names are a generation
  behind. A model that is neither `gemini-*` nor `inherit` is an advisory
  finding — Gemini falls back to its own.
- **Command file:** a TOML table with `description` and `prompt`, written
  through the TOML serializer so a body full of quotes cannot break out of
  the value. The generated-file banner sits outside the prompt as a `#`
  comment rather than being read aloud every run. Only `.toml` loads from the
  commands directory, which is what makes the `.disabled` rename toggle safe
  there (`crates/core/src/render/command.rs`).
- **Tool vocabulary:** `read_file`, `grep_search`, `glob`, `list_directory`,
  `run_shell_command`, `replace`, `write_file`, `web_fetch`,
  `google_web_search`, `write_todos`, `ask_user`. An unmapped name passes
  through so an MCP tool keeps its own id; Gemini then does not offer it,
  which is narrower, never wider. Six of the eight mappings in general
  circulation are wrong, and a wrong one drops the tool in silence — hence
  the table (`crates/core/src/render/vocab/mod.rs`).

## Permissions

`tools:` is a real allowlist, so an `AllowOnly` intent renders natively and
nothing has to be complemented. A `DenyExtra` intent cannot be expressed:
Gemini's agent frontmatter carries an allowlist and nothing else, and
completing one from a deny list would take the agent's own tools away the
moment Gemini grows a built-in it never named. The rendering warns, names the
tools the agent keeps, and installs.

## Hooks

Enforced: 11 events, regex matchers over tool names, exit codes honored.

| Fleet event | Gemini event |
|---|---|
| `PreToolUse`, `BeforeTool` | `BeforeTool` |
| `PostToolUse`, `AfterTool` | `AfterTool` |
| `PreCompact`, `PreCompress` | `PreCompress` |
| `SessionStart` / `SessionEnd` / `Notification` | same |
| `BeforeModel` / `AfterModel` / `BeforeToolSelection` / `BeforeAgent` / `AfterAgent` | same |

An event with no counterpart is left unmapped and nothing is registered, with
a note saying why — a safety hook on the wrong event is worse than one the
user is told did not install.

**Timeouts are milliseconds.** The source declares seconds; the registration
multiplies by 1000 (Gemini's own default is 60000). The script lands at
`<root>/hooks/<name>.sh` — a directory Gemini does not scan, so nothing reads
it except the command registered in `settings.json`. At project scope the
command resolves through `$(git rev-parse --show-toplevel)`, since Gemini
documents no project-directory variable.

A matcher carrying regex syntax around a tool name is registered exactly as
authored and reported, because a matcher that never matches is a protection
that never runs.

## Effective state — when an install is inert

- **`experimental.enableAgents: false`** — agents install and stay inert.
  Absence is not the feature being off; Gemini's own default is on.
- **The system settings layer outranks project scope.** Precedence, later
  wins: defaults → system defaults → `~/.gemini/settings.json` →
  `<project>/.gemini/settings.json` → **system settings** → environment →
  flags. The system file lives at `/etc/gemini-cli/settings.json`,
  `/Library/Application Support/GeminiCli/settings.json` on macOS, or
  `C:\ProgramData\gemini-cli\settings.json` on Windows, relocatable by
  `GEMINI_CLI_SYSTEM_SETTINGS_PATH`. When it defines a key vstack is about to
  write (`agents`, `hooks`, `mcpServers`), the plan warns that what vstack
  writes can be overridden.
- **`mcp-server-enablement.json`** — one global file, whatever scope declared
  the server. A server switched off there is declared for the project and
  inert; a project cannot turn it back on.
- **`mcp.excluded` / `mcp.allowed`** — a server named in `excluded`, or
  absent from a non-empty `allowed`, is kept out of the list Gemini loads.
  Both this scope's settings and the user's are asked.

All four are reads of files on disk, so the wording says how things are
configured and never claims what a run will do
(`crates/core/src/engine/gemini.rs`,
`crates/core/src/harness/gemini/settings.rs`).

## Migration and old-shape tolerance

Gemini's `settings.json` moved to a nested schema in CLI v0.3.0. A file
holding none of the 25 known top-level categories has never been through a
CLI that reads the current shape, so it is treated as legacy and every
settings-backed write is refused with a reason naming the flat pre-v0.3.0
keys. An absent or empty file counts as current — a write creates it in the
current schema. A file that will not parse also reads as current: the
structured-edit path parses it again and reports the failure against its own
path, which is a better error than a shape guess.

## Cross-reads

Gemini reads `.agents/skills`, the shared tree Codex and Pi own. The adapter
does not claim it — the reach is reported as a note so one file on disk is
counted once (`cross_read_note`, `crates/core/src/engine/desired_skill.rs`).

## Where the code diverges from the research

- The research listed `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` alongside the
  settings path. Only `GEMINI_CLI_SYSTEM_SETTINGS_PATH` is read: the defaults
  layer sits *below* user scope and cannot make a vstack write inert, so
  reading it would buy nothing.
- The research recommended preferring `inherit` wherever a tier is
  unspecified, since the 3.x ids carry a churning `-preview` suffix. Shipped
  behavior pins the tiers and reserves `inherit` for an explicit request, so
  a declared tier means the same thing on Gemini as everywhere else.
- Gemini's documented subagent frontmatter also accepts `mcpServers`,
  `temperature`, `max_turns` and `timeout_mins`. vstack writes none of them.
- `kind: remote` subagents are observed like any other file. vstack always
  writes `kind: local` and never installs a remote one, but the scanner does
  not filter remote agents out of what it reports.
