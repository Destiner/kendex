# Changelog

Notable changes, per [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are written when a change lands, not batched at release. Breaking
changes carry a **Breaking** call-out with their migration note inline.

## [Unreleased]

### Added

- GitHub Copilot is now fully managed — agents, skills, hooks, and MCP
  servers install, switch on and off, and come off disk like every other
  tool, personally and per project. Each lands where Copilot actually
  reads it: agents as `.agent.md` files with a tools allowlist in
  Copilot's own tool names, skills in its skills folder, hooks as a hook
  file of their own that Copilot runs and honors the result of, servers
  keyed the way Copilot expects with the transport named on the entry.
  Copilot has no slash commands of its own, so vstack does not invent
  any. Because Copilot reads other tools' files too, three things are now
  said out loud rather than left to surprise you: a skill installed for
  Claude Code is reported as something Copilot already sees — one
  definition, never counted twice; a hook installs but is reported as
  doing nothing when hooks have been switched off anywhere Copilot looks,
  including Claude Code's own settings; and a skill or server your
  personal Copilot settings hold down is reported as something this
  project cannot switch back on, because Copilot only ever lets a
  repository add to that list. An agent pinned to a model the repository's
  allowed-models list refuses is flagged the same way. Which model Copilot
  uses is left to Copilot: its list changes monthly and depends on your
  plan and your organization, so vstack pins nothing it cannot promise.

- **Breaking:** a plugin now belongs to one tool. Copilot and Claude Code
  both keep a list of enabled plugins, and a declaration that named
  neither used to be written into every tool's settings — switching on
  software in one tool because it was installed in another. Every plugin
  declaration now carries the tool it belongs to. *Migration:* existing
  declarations are read as Claude Code's, which is the only tool vstack
  ever wrote a plugin switch for, and the next save records that in
  `vstack.toml`; nothing to change by hand. Add `harness = "copilot"` to
  a plugin declaration to aim it at Copilot instead.

- Gemini CLI is now fully managed — agents, skills, commands, hooks, and
  MCP servers install, switch on and off, and come off disk like every
  other tool, personally and per project. Each lands in the shape Gemini
  actually reads: agents as its own subagent files naming its own tools
  (`read_file`, `run_shell_command` — not the names other tools use),
  commands as Gemini command files, hooks registered under Gemini's own
  event names with the timeout in the units it reads. Two things about
  Gemini are said plainly instead of glossed over: whether an MCP server is
  switched on is recorded once for the whole machine, so a project can
  bring a server in but has to remove it rather than switch it off there;
  and an agent installed while Gemini's subagents are turned off is
  reported as installed-but-doing-nothing rather than as ready. Where the
  installed Gemini is older than the settings file vstack writes, or where
  a machine-wide settings file outranks what vstack puts in a project,
  vstack says so and leaves the file alone instead of writing something
  that would never be read. Gemini's extensions stay read-only: they
  install in one place for the whole machine and switch on through a rules
  file nobody has documented.

- vstack now sees Gemini CLI and GitHub Copilot setups, personally and per
  project, listed beside every other tool: Gemini's agents, skills,
  commands, hooks, MCP servers, and extensions, and Copilot's agents,
  skills, and MCP servers. Copilot's folder is found where Copilot
  actually keeps it, including a relocated one. Files the two tools borrow
  from each other, like Copilot reading Claude Code's skills, stay listed
  once under the tool they belong to instead of being counted twice.

- Every generated file is checked against its tool's real format before
  anything is written. A file that tool would not load — an unparseable
  Codex agent, an OpenCode agent whose mode or permissions it cannot read,
  a skill whose SKILL.md names a different skill than the folder it sits
  in, a name OpenCode's loader rejects like `My_Skill` — is blocked in the
  plan with the fix spelled out, instead of installing broken and going
  quiet. Only the tool that rejects it is blocked: the same item still
  installs everywhere its format is valid — except where tools read the
  same folder, where one file serves them all, so a refusal there covers
  every tool reading it. Files that load but not as written, like a Cursor
  rule carrying keys Cursor ignores, install with a warning rather than a
  block.

- **Breaking:** commands install on Codex, which retired its prompt
  directory in favor of skills. A declared command lands on Codex's skill
  surface as a generated skill — frontmatter, the generated-file banner,
  then the command body — at both scopes, and it toggles and comes off
  disk there like any skill. The install record keeps the name and paths
  the command actually took, so removal and refresh target what was
  written. A command whose name a skill already holds installs as
  `<name>__command`, or `<name>__cmd` when that is taken too, with a
  warning naming it. OpenCode and Cursor still only read commands.
  Migration: refresh creates these — no Codex command artifacts existed
  before, and `~/.codex/prompts` is still never written to.

- Agent instructions now speak each tool's own vocabulary. A body written
  in Claude's words — "use the Read tool" — is reworded as it installs on
  OpenCode, Cursor, and Pi, so the agent reading it gets an instruction
  about a tool it actually has instead of a name it does not recognize.
  Codex is narrower, because it names actions rather than tools: only a
  whole "use the Read tool" becomes "open the file", and every other
  mention stays as authored, since an action phrase dropped into a name's
  place turns the sentence into nonsense. Only unmistakable references are
  touched: code samples, links, generated skill paths, backtick-quoted
  names on Codex, and the project's own launch and additional instructions
  keep every byte. A custom or MCP tool name is never guessed at — it
  passes through as written, and the plan preview names both what was
  reworded and what was left alone.

- Catalog downloads are hardened against the repositories they fetch. A
  source repository can no longer redirect a refresh at files outside its
  own cache, no git call can stall the app waiting on a credential or SSH
  prompt, and every external command gives up with an error rather than
  hanging forever.

### Changed

- **Breaking:** agent tool permissions are typed intent, preserved from
  source to every renderer and never widened. A missing `role:` no longer
  renders Codex `sandbox_mode = "danger-full-access"` (role-less agents get
  the sandbox their `tools:` list justifies); a source `tools:` allowlist
  renders natively on Claude, synthesizes an OpenCode permission block,
  infers the Codex sandbox, and is refused on Pi where honoring it is
  impossible; the v1 importer carries legacy `tools:` allowlists over as
  `allow-tools` overrides instead of dropping them. Migration: refresh
  regenerates installed agents; an agent that wants full access declares
  `role: engineer` explicitly.

- **Breaking:** model aliases resolve through one per-harness table
  (`fable`, `opus`, `sonnet`, `haiku`, `inherit`); `inherit` now survives
  every harness (OpenCode/Codex/Pi omit the field instead of emitting an
  invalid id such as `openai/inherit`), explicit vendor ids pass through
  untouched, and a bare unknown model warns where the harness's loader
  requires a `provider/model` form. Migration: refresh regenerates.

### Fixed

- Bringing a Gemini MCP server into a project no longer switches that
  server back on for your whole machine. Gemini records whether a server
  is on in one file every project shares; a project now reads that file
  and says "declared here, but switched off for this machine" instead of
  rewriting it, and removing the server from a project leaves the
  machine-wide switch exactly where you set it. Switching a server on and
  off personally works as before.
- A safety hook written for Claude Code now matches on Gemini CLI and
  GitHub Copilot. Hooks name the tool they guard — "Bash" — and each tool
  has its own name for it, so the name is translated on the way in
  (`run_shell_command` on Gemini, `bash` on Copilot); before, the hook
  installed looking correct and never fired. A matcher vstack cannot
  translate — a regular expression rather than a plain name — installs
  exactly as written and is flagged as possibly matching nothing.
- Installing a safety hook on Cursor or OpenCode now says plainly that
  neither tool runs hooks: the plan marks it "(advisory)" and the report
  says it lands as text the model may ignore. Every tool's card also says
  whether it runs safety hooks at all.
- `vstack verify` no longer prints a clean tick for an installation that
  cannot do anything — a hook switched off machine-wide, a server Gemini
  gates out, an agent installed while subagents are off. The reason is
  printed beside the row; it still does not fail the run, because nothing
  is wrong with what was installed.
- A skill named in a way GitHub Copilot will not load is refused with the
  spelling that works, instead of installing where it is never listed.
- A skill installed for another tool is now reported as visible to Gemini
  CLI as well as to Copilot — both read the shared skills folder, and
  neither gains a phantom installation of its own.
- An item declared only for tools that cannot hold it — a slash command
  for Copilot, which has none — now says so instead of silently
  installing nowhere.
- One unreadable Pi package no longer empties the whole `update-pi`
  listing — it gets its own note and the healthy rows still print.
- A symlinked configuration file inside a catalog is refused loudly
  instead of being silently treated as absent, and plan rows for
  settings changes name the tool again.
- **Breaking:** a skill too large for Codex's loader now splits into a
  head plus `references/details.md` instead of silently truncating at
  load; tools without the cap keep the whole body on their own copy, and
  a command that installs as a Codex skill splits the same way. Nothing
  is refused for size unless the split itself is impossible — a single
  code block spanning the limit — and the message says so. Migration:
  refresh regenerates.
- The editor's skill list no longer breaks on a machine where nothing
  was ever adopted; the reserved local source reads as missing until
  adopt creates it.
- A catalog item refused for a hostile read now fails `vstack verify`
  and `vstack refresh` instead of printing a green tick.
- OpenCode agents pinned to a bare vendor model id keep loading: the id
  gains OpenCode's default `openai/` provider prefix as before.
- A project's identity no longer depends on how its path was spelled:
  the writer lock and every derived path key off the canonical root, so
  two differently-written paths to one project can never write
  concurrently.
- Two or more settings changes to the same configuration file now apply
  together in one write: installing two MCP servers (or a hook plus a
  server, or any mix of registrations and removals) into one settings
  file in one apply used to fail and roll back; each file now gets a
  single composed mutation with a single precondition.
- A tool refusing a skill no longer wedges the project. Where two tools
  read the same folder, the refusal used to plan the same removal twice,
  which failed and rolled the whole apply back — nothing in that project
  could be applied again until the catalog was hand-edited. The refusal
  also no longer takes the folder away from a tool that accepted the
  skill and is still reading it. Two tools pointed at one folder likewise
  no longer plan the same connection twice.
- A skill that grows past a tool's size limit, or shrinks back under it,
  now moves cleanly between the shared copy and a copy of its own. Tools
  with no limit used to keep reading the shortened copy through a stale
  link — exactly the truncation splitting exists to prevent — and the
  change was reported as a conflict with nothing the user could do about
  it.
- Two commands can no longer claim one generated name. Names are handed
  out in a fixed order, so a command keeps the same name from one check
  to the next instead of two commands swapping bodies on every apply.
- A command whose name a skill takes over, or gives back, no longer
  leaves its old copy behind for the tools to offer under a name nobody
  declared.
- A long skill now splits at any section heading, not only top-level
  ones, so what the tool reads stays a skill instead of becoming a
  pointer. A code block indented inside a list item is recognized as
  code: it is never cut through and never reworded.
- A command too long for Codex splits like any other skill instead of
  being refused with a fix its author could not make, and the plan says
  when the generated skill also lands where Pi reads.
- A command's one-line summary comes from its own `description`, not from
  one nested under another key, and its frontmatter no longer appears as
  literal text inside the generated skill.
- A custom `GIT_SSH_COMMAND` is extended rather than replaced, so a
  catalog that needs a particular SSH key keeps fetching.
- A command that outlives its timeout now takes everything it started
  with it, instead of leaving a stray process running behind it.

### Changed

- **Breaking:** the manifest schema and install-record version move to 2.
  v0.1 files still load; the first apply upgrades them in place through
  the normal journaled, previewed plan — the upgrade changes the schema
  line and nothing else, and an interrupted upgrade rolls back
  byte-identically. Files written by a newer vstack refuse to load
  instead of being corrupted. Migration: automatic on first apply;
  `vstack import` still covers v1.

### Added

- **Breaking:** installed skills follow the surface model: tools that
  read the same folder (Codex and Pi share `.agents/skills` in a
  project) get exactly one copy rendered to their combined limits, and a
  tool whose copy must differ gets its own — identical copies still
  collapse onto one tree through links, so today's layout is unchanged.
  Migration: refresh regenerates; the journaled apply moves anything
  that needs to move.
- Render and parse warnings are now first-class: each names its item and
  tool, says what happened, and carries the fix when there is one —
  shown in the plan preview, the Sync page, and every CLI verb that
  prints a plan.
- Every catalog read goes through one sealed API: reads resolve against
  the canonical source root, symlinks in a catalog are refused loudly (a
  hostile catalog can no longer pull host files into generated artifacts
  or recurse forever), and traversal carries depth, count, and byte
  budgets. One refused item degrades to a note; the rest of the scope
  still plans.
- Source frontmatter is parsed as real YAML (block scalars, arrays, nested
  maps) with adversarial-input bounds: aliases, duplicate keys, oversized or
  deeply nested frontmatter are refused, and unknown keys warn instead of
  silently vanishing.

## [0.1.0] - 2026-08-10

First v2 release: desktop app (Tauri) + `vstack` CLI over one engine,
replacing vstack v1.

### Added

- Scan → declare → diff → apply engine over per-scope `vstack.toml`
  manifests, with preview-first, journaled, transactional applies and
  crash recovery; removals go to a trash, never a hard delete.
- Five harnesses — Claude Code, Codex, OpenCode, Cursor, Pi — behind one
  adapter seam with a single capability table gating every operation.
- Agents and skills authored once, rendered per tool; hooks, commands,
  MCP servers, plugins, and Pi extensions managed where each tool
  supports them.
- Catalog sources as plain git repos or local paths, enabled per scope;
  adopt brings hand-made files under management.
- CLI verbs mirroring every core operation: `add`, `remove`, `adopt`,
  `apply`, `refresh`, `verify`, `list`, `check`, `source`, `project`,
  `report`, `update`, `update-pi`, `import`, `init`.
- Self-updating app and CLI via a tag-driven draft-release feed.

### Breaking

- **Breaking:** fresh manifest and lock schema; v1 files are not read.
  Migration: `vstack import` converts v1 manifests and locks in place
  (originals copied to the trash first), then `vstack refresh`
  regenerates every installation.
- **Breaking:** v1 extras and theme packs are not carried over.
