# GitHub Copilot

Copilot is four products sharing filenames. vstack treats **Copilot CLI plus
repository files** as the harness and ignores the rest — a file only VS Code
reads is not something a CLI-shaped adapter should claim to manage. It also
reads more configuration than it owns, which is the single biggest modelling
constraint here.

Facts below were verified against docs.github.com and code.visualstudio.com,
accessed 2026-08-10 (roots, kinds, precedence, models) and 2026-08-13 (hook
events, hook file shape, tool names).

## Roots

| Scope | Path | Relocated by |
|---|---|---|
| Global | `~/.copilot` | `COPILOT_HOME` — it relocates the whole config root |
| Project | `<project>/.github` | — |

Project markers: `.github/copilot-instructions.md`, or a `.github/agents`,
`.github/skills` or `.github/hooks` directory. `.github/` on its own is not a
marker — nearly every repository has one. Owner:
`crates/core/src/harness/copilot/mod.rs`.

## Surfaces

| Kind | Global | Project | Caps |
|---|---|---|---|
| agent | `~/.copilot/agents/*.agent.md` | `.github/agents/*.agent.md` | managed, both |
| skill | `~/.copilot/skills/<name>/SKILL.md` | `.github/skills/<name>/SKILL.md` | managed, both |
| hook | `~/.copilot/hooks/*.json` (each file a document), plus `~/.copilot/settings.json` → `hooks` | `.github/hooks/*.json`, plus `.github/copilot/settings.json` and `settings.local.json` → `hooks` | managed, both, **enforced** |
| mcp-server | `~/.copilot/mcp-config.json` | `.github/mcp.json` | managed, both |
| plugin | `~/.copilot/settings.json` → `enabledPlugins` | `.github/copilot/settings.json` + `settings.local.json` → `enabledPlugins` | observe + toggle, both |
| command | — | — | unsupported |
| pi-extension | — | — | unsupported |

**Commands are unsupported because no file-backed slash-command kind exists
in any Copilot product.** The nearest thing, prompt files
`.github/prompts/*.prompt.md`, is IDE-only, in public preview, and read by
neither the CLI nor github.com.

**Hooks are read from two places and written to one.** Every `*.json` under
the hooks directory is a whole `{version, hooks}` document of its own, and the
settings file carries a `hooks` key of the same entries. Both are observed;
only the files are written, because an entry inline in a settings file has no
switch of its own to flip and removing it to disable would be a lossy toggle.

**Plugin install and remove are parked** with the Claude marketplace work.
`enabledPlugins` is a clean boolean flip, so the toggle ships; installing one
needs marketplace resolution vstack cannot do yet.

## Format facts

- **Byte cap:** none.
- **Name rule:** `LowerKebab { max_len: None }` — a SKILL.md `name` is
  required in lowercase-hyphen with no documented length. The namespace
  separator is therefore `-`, not `__`.
- **MCP transports:** stdio, streamable HTTP, SSE. A command server is typed
  `local`; a url server keeps whatever transport it declares, named by `type`
  (`server`, `crates/core/src/engine/copilot.rs`).
- **Agent file:** `<name>.agent.md` — the double extension is part of what
  the loader looks for, not decoration. YAML frontmatter + markdown body.
  vstack writes `name`, `description`, `model?` and `tools`. Skills and hooks
  are not frontmatter fields, so both travel as prose
  (`crates/core/src/render/agent/copilot.rs`).
- **Model dialect:** every tier resolves to `auto`, and `inherit` omits the
  key entirely. Copilot's model list moves monthly and is gated by
  subscription, org policy and a per-repository allowlist, so vstack pins
  nothing. An explicit user-set id passes through unchanged and is surfaced as
  free text, never validated against an enum.
- **Tool vocabulary:** `read`, `grep`, `glob`, `bash`, `edit`, `multiedit`,
  `write`, `webfetch`, `websearch`, `todowrite`, `agent`, `notebookread`,
  `notebookedit`. A name Copilot does not document is left alone rather than
  guessed at — an allowlist entry it does not recognize grants nothing, which
  is narrower than asked for, never wider.
- **Agent scoping:** none — a registered hook cannot tell which agent
  triggered it (it does fire `subagentStart`/`subagentStop`, which is what
  the payload research would have to read), so only `agents = "all"`
  custom hooks are enforced here.

## Permissions

`tools:` is a real allowlist, so an `AllowOnly` intent renders natively. A
`DenyExtra` intent cannot be expressed — Copilot's agent frontmatter carries
an allowlist and nothing else — and completing one from a deny list would
take the agent's own tools away the moment Copilot grows a built-in it never
named. The rendering warns, names the tools the agent keeps, and installs.

## Hooks

Enforced: Copilot runs the command and honors the exit code.

| Fleet event | Copilot event |
|---|---|
| `PreToolUse` | `preToolUse` |
| `PostToolUse` | `postToolUse` |
| `PermissionRequest` | `permissionRequest` |
| `UserPromptSubmit` | `userPromptSubmitted` |
| `SessionStart` / `SessionEnd` | `sessionStart` / `sessionEnd` |
| `PreCompact` | `preCompact` |
| `Notification` | `notification` |
| `Stop` | `agentStop` |
| `SubagentStop` | `subagentStop` |

Copilot accepts a PascalCase spelling of each name too; the camelCase one is
what its reference writes, so that is what vstack registers. Its remaining
events — `postToolUseFailure`, `userPromptTransformed`, `subagentStart`,
`errorOccurred` — have no fleet counterpart and stay unmapped, with a note
rather than a near-miss.

**Timeouts are seconds** (`timeoutSec`), so they travel as the source wrote
them. Copilot loads every `*.json` under its hooks directory as a document of
its own, so each hook gets a file of its own — `<name>.json` beside
`<name>.sh`, which the glob does not see. The document shape is
`{"version": 1, "hooks": {"<event>": [{"type": "command", "bash": …,
"matcher": …, "timeoutSec": …}]}}`; a file left holding no hooks keeps its
version line, because it is still a hook file
(`crates/core/src/configedit/copilot.rs`).

At project scope the command resolves through
`$(git rev-parse --show-toplevel)`.

## Effective state — when an install is inert

- **`disableAllHooks`** switches off every Copilot hook, all or nothing.
  vstack reads the whole layer stack, lowest first, and reports which file
  threw the switch: legacy `~/.copilot/config.json` → `~/.copilot/settings.json`
  → `.claude/settings.json` → `.claude/settings.local.json` →
  `.github/copilot/settings.json` → `.github/copilot/settings.local.json`.
  Later wins, so a repository can switch hooks back on over a personal
  disable.
- **`disabledSkills` / `disabledMcpServers` in a personal file.** Only a
  fixed allowlist of keys is honored at repository scope, and several merge as
  a union: a repository may *add* a name to a disabled list but can never take
  one off. A project-scope enable over a user-scope disable is therefore not
  expressible, so vstack does not write one — it reports the hold per item,
  naming the file and the key to edit.
- **`.github/allowed_models.txt`** restricts model ids with `*` globs (a
  `fallback:` line names what to use when nothing matches and is not itself a
  pattern). An agent naming a model outside the list warns; `auto` is a
  routing mode rather than an id, so the allowlist has nothing to say about
  it.

All three are reads of files on disk, so the wording says how things are
configured and never claims what a run will do
(`crates/core/src/engine/copilot.rs`,
`crates/core/src/harness/copilot/settings.rs`).

## Migration and old-shape tolerance

Copilot moved its user-editable settings out of `config.json` into
`settings.json`. The old file is read so an older machine is understood, and
never written. A global scope still holding `config.json` with no
`settings.json` has never run a CLI that reads what vstack would write, so
settings-backed writes there are refused with that reason rather than left
somewhere nothing loads them.

## Cross-reads — Copilot reads other tools' files

Copilot CLI discovers skills from `.claude/skills` and `.agents/skills`, VS
Code discovers agents from `.claude/agents`, and the CLI reads
`.claude/settings.json` and `.claude/settings.local.json` for
`companyAnnouncements`, `disableAllHooks`, `enabledPlugins`,
`extraKnownMarketplaces` and `hooks`.

The adapter claims none of them. `Installation = item × harness × scope`
would double-count otherwise: one file on disk would be two installations,
and a removal offered against one would take the other tool's copy away. The
same rule keeps a repo-root `.mcp.json` out of the surface list — it is
Claude Code's file, and evidence of MCP rather than of Copilot. Where the
reach matters it is reported: as a note on the plan for skills, and as the
`disableAllHooks` layer stack above.

## Where the code diverges from the research

- The research recommended the capability table carry the repository-scope
  asymmetry (`disabledSkills` disable-only at project scope) as a column.
  It does not. vstack's own switch is a rename it can undo either way, and a
  column saying otherwise would forbid a working enable — so the external hold
  is reported per item, where it is read.
- The research recommended a file-rename toggle for `.github/hooks/*.json`.
  Shipped behavior writes one document per hook and toggles by renaming the
  *script* to `.disabled` while reversing the registration inside the JSON
  document, which is the same mechanism every other harness uses.
- The research recommended emitting `auto` when no tier is given. Shipped
  behavior maps *every* tier to `auto` and omits the key for `inherit`, since
  which models a user can reach depends on their plan and their organization.
- `user-invocable` and `disable-model-invocation` are documented agent
  fields, not skill fields. vstack writes neither on either.
