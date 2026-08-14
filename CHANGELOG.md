# Changelog

Notable changes, per [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are written when a change lands, not batched at release. Breaking
changes carry a **Breaking** call-out with their migration note inline.

## [Unreleased]

### Added

- Everything you install is now read before it lands, and again after.
  Two separate scores come out of it, and they are never mixed together.
  The safety score answers "could this hurt me" — content that tells the
  model to ignore its instructions, a line that downloads a script and
  runs it, a command that reads your SSH key and sends it somewhere, a
  real credential left in a file, a server launched from a package name
  anyone could have registered. The quality score answers "is this well
  written" — does it say when to use it, is the detail behind a pointer
  instead of all in the front door, would it read the same on another
  tool. Only safety can hold anything back; quality is there to inform.
  Every finding names the file and line, says what it found in plain
  words, and comes with the fix. A command inside a code block in the file
  a tool actually loads counts in full — that is where skills put their
  commands, and the model reads them either way. What counts for less is
  writing that is plainly quoting rather than telling: a blockquote, and
  the test fixtures and reference pages a skill ships alongside it.
  Credentials count the same wherever they are. A leaked key is never
  repeated anywhere: you get a fingerprint,
  enough to tell two leaks apart and useless to anyone who sees it. Text
  carrying hidden characters, or letters chosen to look like other
  letters, is reported as such — content that needs decoding to look
  clean has told you something.
- `vstack check --catalog <dir>` validates a catalog the way an install
  would, and exits non-zero when something is wrong — so a repository can
  find out in its own CI rather than in someone else's install preview.
  It checks both halves: whether each tool's loader could actually hold
  the item (a name it will not accept, a SKILL.md that disagrees with its
  own folder, a body past the tightest size cap) and whether the content
  is safe. `--strict` also fails on advice. A reusable GitHub Actions
  workflow ships with it — catalog repositories point one line at
  `.github/workflows/catalog-check.yml` and get the gate. What `vstack
  init` scaffolds passes it on the first run.
- Install a whole set at once. A catalog can offer named bundles — a
  starter kit, a review workflow, the tools one team shares — and
  installing one brings in every agent, skill, command and hook it
  carries: `vstack add <catalog> --bundle <name>`, or the Install button
  now beside each catalog on the Catalogs page. Repositories that ship
  marketplace-style plugins need no extra authoring, because each plugin
  is already a set, with the version and description it publishes.
  Uninstalling is the half that usually goes wrong, so it says exactly
  what it will do before it does it: members you also asked for by name,
  members another installed bundle carries, and members something else
  still needs all stay, everything else goes, and every line comes with
  the reason it went or stayed. Removing a single member sticks too — a
  refresh will not quietly put it back, and the audit reports the bundle
  as installed with members held back rather than as complete. When a
  catalog changes its mind about what a bundle carries, the additions and
  removals appear in the refresh preview and wait for an answer before
  anything is installed or uninstalled.
- Skills can require other skills: a required companion installs with
  its parent (for the tools that support it, with a warning where one
  cannot), an optional companion is a real install-time choice that
  survives refresh and other machines, and removing something warns
  about what still needs it — with an optional sweep of leftovers
  nothing needs anymore. Removing a required companion sticks: it stays
  removed across refreshes and the parent shows a "missing required
  dependency" note instead of it silently coming back.
- **Breaking:** every installation records the reasons it exists —
  asked for directly, required by another item, or part of a bundle —
  and those reasons drive removal decisions. Migration: existing
  install records gain a single "asked for directly" reason, the only
  safe reading.
- **Breaking:** installing can now be refused. Anything the safety check
  rates as critical is held back on its own, and so is anything whose
  overall score falls below 60; between 60 and 80 it installs and warns.
  A held-back item shows up the way any other conflict does — it appears
  in the preview with what was found and why, and nothing about it is
  written. Migration: the two thresholds are yours to set in app
  settings, and nothing else changes for content that passes. If you have
  read the findings and want it anyway, the preview prints the exact
  command that installs it — `vstack apply --allow-unsafe <name>@<code>`,
  where the code stands for the content you were just shown — and records
  the review in your `vstack.toml`. The name on its own does nothing, so a
  line left in a script or a shell history cannot wave through content
  nobody has read. The record is bound to the exact content, the exact
  rules, and the exact problems you were shown; change any of them and it
  stops applying, the item is held back again, and the preview prints the
  new code. The record lives with the project rather than in a global list
  precisely so it cannot quietly become a permanent exemption.
- **Breaking:** `vstack refresh` no longer changes what is installed
  without asking. Regenerating what is already installed stays
  automatic; anything being added or removed (including dependencies a
  catalog gained or dropped) is shown first and needs confirmation or
  `--yes`. Scripts add `--yes`; a non-interactive run refuses before
  touching anything.

- **Breaking:** vstack can now be pointed at a marketplace-style
  catalog — a repository that ships its content one plugin at a time,
  with a `marketplace.json` listing what it offers — and install straight
  from it, alongside the plain catalogs it has always read. Nothing is
  guessed: a repository is read that way only when it carries that
  listing, and only the plugins the listing describes, kept inside the
  repository itself, are offered. An entry that points at some other
  repository or a web address is skipped and named, rather than quietly
  fetching something nobody asked for. Anything the catalog gets wrong is
  reported with what to do about it: a listing that does not parse, a
  plugin whose own details disagree with the listing about its name or
  version, a plugin describing files outside itself, and two names your
  filesystem cannot tell apart. Items from these catalogs are listed under
  the plugin they came from, so two plugins can each ship an `analyzer`
  without one hiding the other. *Migration:* nothing changes for catalogs
  already in use — a catalog with no listing installs exactly where it
  always did, under the names it always used. Items from a marketplace
  catalog are declared and shown as `<plugin>/<item>` (in `vstack.toml`,
  write the name in quotes), and each tool spells that its own way in the
  files it reads: `data-science__eda` for most, `data-science-eda` where
  the tool only accepts lowercase words joined by hyphens. If two
  declarations would end up as the same file — a namespaced name against a
  flat one already spelled that way, or two names that differ only by
  capitals or by how an accent is typed — neither is installed, and the
  conflict names both so you can rename one. Only agents, commands and
  skills come from these catalogs, so only those carry a plugin in their
  name: a hook or an MCP server is still written without a `/`, and a name
  that cannot be a file at all is refused when `vstack.toml` is read.

- **Breaking:** a source can now say which revision it reads, and
  downloaded catalogs are kept one folder per version instead of one
  working copy per repository that every refresh reset in place. Add
  `rev = "<commit, tag or branch>"` to a source in `vstack.toml`, or name
  it when adding one as `owner/repo@<rev>`. A full commit id is a pin —
  that exact content, forever, and it keeps working with no network once
  it has been downloaded. A tag or branch is followed instead: each
  refresh re-resolves it, and a tag that moved upstream shows up as a
  pending change to preview like any other, never as a silent rewrite.
  Two projects can now sit on different versions of the same catalog at
  once, and a refresh started in one window can no longer change files
  another window is reading. Being offline with a pin that was never
  downloaded is an error naming the pin; everything already installed
  keeps working. *Migration:* the download cache rebuilds itself on the
  next refresh — no user content is involved — and the old cache folders
  are left in place, still readable, rather than deleted. The new layout
  keeps one folder per version it has read, which is what lets two
  projects sit on different ones; a catalog you follow by branch therefore
  gains a folder each time it changes upstream, and nothing tidies them up
  yet. Deleting the whole cache folder is safe whenever it gets large — it
  is rebuilt on the next refresh. Nothing in `vstack.toml` has to change:
  a source with no `rev` follows its repository's default branch exactly
  as before.

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

- The building blocks behind every control — menus, dialogs, switches,
  tabs, tooltips — were swapped for Base UI, their maintained successor.
  Nothing looks or behaves differently day to day; dropdown labels,
  keyboard navigation, and focus behavior were verified page by page
  against the previous build. The one deliberate change: arrowing
  through tabs now moves focus without switching the tab until you press
  Enter, which matches how tabs work across current apps.

- Keyboard focus now looks the same on every control: buttons and tabs
  had kept an older, heavier focus outline than the rest of the app, and
  the checkbox's error outline was too faint in dark mode.

- Home earned its place: what needs attention leads, a new "Recent
  activity" list shows the latest-changed items on your machine (each
  row jumps to the Library, filtered), and the count tiles moved
  below as an at-a-glance strip. The stray error line at the bottom
  is gone — errors show where they happen now.

- Review & apply explains itself now — "Changes vstack wants to make,
  and things it found; nothing touches your files until you apply" —
  and the page reads in order of urgency: held-back items in a tinted
  panel that is unmistakably first, then the changes applying would
  make, then safety notes worth a look, then items not managed yet
  (with a line on what managing does), then the all-clear. Clicking
  "Start managing" confirms itself ("Now managing …") instead of the
  row silently vanishing.

- Clicking an item in the Library opens a proper detail panel: close
  it with the X, Escape, or another click on the row. It shows the
  item's type, tools, where it lives, its file path, when it last
  changed, and where it came from — plus the item's own content,
  rendered nicely for text and shown as code for scripts — and a
  "Show in file browser" button opens the folder on disk. The table
  itself gained type icons, a "Where" column and filter (Personal or
  per project), a quiet "Updated" column, and one rule for the line
  under each name: it is always the item's description, never a
  version or commit hash — those now live in the panel and read as
  data, not prose. Both Library tabs share one content width.

- Anywhere a folder path can be typed — adding a project, scanning
  for projects, tool-folder overrides — a Browse… button now opens
  the system folder picker instead of making you type the path.

- Section headings inside cards got a real hierarchy: a small quiet
  label above the content instead of a heading the same size as
  everything else, with row titles and descriptions on a consistent
  scale across Settings and Tools & Projects.

- The app draws its own title bar now — the system frame is gone.
  Window controls sit top-right in the app's own style, the top edge
  is a drag handle (double-click to maximize), and the whole window
  looks the same in both themes instead of wearing the desktop's
  frame. The controls float inside the page rather than taking a bar
  of their own, so content starts higher, and the heavy divider lines
  under the old bar and under tab strips are gone.

- A quiet status strip runs along the bottom of the window: whether
  the last scan is current ("Up to date · scanned 2m ago"), and — when
  there's something to do — how many changes are pending and how many
  installs are held back, each a click away from Review & apply.

- The back arrow and breadcrumb now appear only after you follow a
  link from one page into another; opening a page from the sidebar
  shows neither.

- You can step back. Following a link across pages — a count on Home,
  a tool's badge into the Library — leaves a quiet back arrow and a
  breadcrumb ("Library / Installed") at the top of the page; clicking
  a section in the sidebar starts fresh.

- Long "affects" lists on Review & apply fold away instead of printing
  a wall of identifiers: you see the count and the first few names with
  a "+17 more" you can expand. And when several findings hit the exact
  same set of items, the set is shown once with those findings stacked
  above it — the same 21 plugin names no longer print twice.

- The app now summarizes instead of listing. Review & apply used to repeat
  an identical warning under every hook it touched — seven hooks sharing
  one settings file meant seven copies — and gave every clean plugin its
  own row; now a finding is said once with the items it affects listed
  under it, clean items collapse to one sentence, and internal
  identifiers and numeric scores stay out of the headlines. Home tells
  the truth at a glance: "changes ready to apply" and "items that aren't
  managed yet" are counted separately (they used to be lumped together as
  "out of date"), and the summary sentence at the bottom became three
  tiles — tools, installed, projects — that take you to the page they
  describe. Every page now shares one content width, tool cards became a
  compact list whose counts click through to the Library pre-filtered,
  the rarely-used folder override moved off every tool card into
  Settings, and inputs are sized to what they hold.

- The app has a considered look now instead of stock defaults: a
  near-black ground with a blue accent, and color that carries meaning —
  green means healthy, amber means worth attention, red means held back,
  blue means an update is waiting. Status dots and tinted pills replace
  the grey-on-grey badges, the one primary action on each screen is the
  one blue button, file paths and versions read in monospace, and both
  the light and dark themes got the same treatment. Pressing `/` now
  jumps to the Library search box, and the box says so.

- The safety check's caution level is a Settings control now (Strict /
  Balanced / Lenient) rather than a threshold with no way to set it.

- The app is reorganized around what you're trying to do, not around its
  internals: six sidebar destinations instead of eight. Home now leads
  with what needs your attention — out of date, held back for safety, or
  otherwise worth a look — each with its fix one click away, and a quiet
  all-clear when there's nothing to do. Sync is now Review & apply, the
  same preview-then-apply screen. Library and Catalogs merge into one
  Library, with Installed and Add from a catalog as its two modes;
  bundles — a catalog's ready-made sets — lead the add flow instead of
  hiding under each catalog's entry. Tools and Projects merge into one
  Tools & Projects, since both answer "where does my setup apply."

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

- The checkboxes in Customize's agent-skills grid were collapsing
  into thin slivers — a leftover from the component-library switch
  that only this grid exercised. They render as proper checkboxes
  again.

- When something fails, you now hear about it where you clicked: a
  small notice appears in the corner with the reason, in plain words.
  Errors used to be easy to miss entirely — adding a project with a
  bad path, for instance, quietly printed its message on the Settings
  page and cleared what you typed. The input keeps what you typed on
  failure now, and adding a project confirms itself with a brief
  "Added" notice.

- Typing a folder the way a terminal spells it — `~/dev/my-project` —
  now works everywhere a path can be typed: adding a project, scanning
  a folder for projects, and tool-folder overrides. Before, the `~`
  was taken literally and the add failed.

- Applying from the app (and `vstack apply`) now performs the "Upgrade
  vstack.toml to the current format" step it promised. Before, the
  preview listed the upgrade but the apply quietly skipped it, so a v0.1
  setup file stayed old forever and the promise came back after every
  apply. Found by walking the real app through the migration, not by the
  test suite — the apply path planned from a copy of the file that no
  longer looked old. The upgrade also now finds the real `schema` line
  even when a comment mentions the same text or the spacing is unusual,
  and changes only that line — comments and formatting survive
  byte-for-byte. Applying a folder whose setup file was deleted out from
  under the preview now says so instead of silently succeeding.

- The safety check no longer flags ordinary code for reading its own
  settings. `process.env.API_URL`, `os.environ[...]`, `import.meta.env`
  and `Deno.env` are how every JavaScript and Python program reads the
  values you gave it, and every one of those lines was being reported as
  reading a credential file — enough of them to hold back any catalog
  with a single JavaScript skill in it. Naming a project's own `.env` in
  a README or opening it in a loader script says nothing either, so it no
  longer says anything. Reading a real key store and sending it somewhere
  — `cat ~/.ssh/id_rsa | curl …` — is still the most serious thing the
  check reports. Sweeping a 39-item catalog now returns twelve findings
  where it used to return two hundred and ninety-six.
- A command shown inside a code block in a SKILL.md is now treated as
  what it is: the instruction. It used to count for less, which meant the
  check held back the awkward way of writing an attack and let through
  the way anybody would actually write one. Test fixtures and reference
  pages that a skill ships alongside itself still count for less, because
  a test asserting on a dangerous command line is describing it, not
  issuing it.
- A single byte that is not text can no longer hide a whole file. Adding
  one to the end of a script used to make it invisible to every rule, and
  the item then scored a perfect hundred on content nobody had read. Such
  a file is now read as far as it can be, and the part that could not be
  read is reported so the score is not mistaken for a clean bill.
- The check now recognises far more letters that are drawn to look like
  English ones. It knew about Cyrillic and Greek capitals; a Greek `υ`, an
  Armenian `ո` or a small-capital `ᴜ` dropped into "ignore previous
  instructions" went through with nothing reported at all.
- Warnings about an MCP server's command line no longer quote the command
  back with an API key still in it. Any value the check repeats to you now
  goes through the same redaction as a key it found on purpose.
- The Audit page tells the truth about items you have accepted. An
  installed item whose findings you read and accepted was being shown as
  "held back" — the opposite of what was true — and an acceptance that no
  longer matched what is on disk was never shown as stale.
- Things the check could not look at now say so instead of disappearing.
  A plugin that is not installed yet, and an MCP server whose entry could
  not be read, used to score a silent hundred out of a hundred and then be
  dropped from the report entirely, so a row nobody had audited read as
  one that passed. MCP servers are now read out of the config file that
  holds them, so most of them are genuinely checked.
- Removing something while its catalog is unavailable now sticks. If the
  catalog was offline, moved, or not downloaded yet, the removal went
  through and then the next refresh quietly put the item back — silently,
  under `vstack refresh --yes` in a script. The removal now stands on what
  vstack already recorded about why the item was installed, so it stays
  removed when the catalog comes back, and the preview says out loud that
  a catalog it could not read may hold consequences it cannot show you.
- An item that is both asked for by name and marked as kept-removed now
  installs and reads as installed. Before, it was installed on disk while
  the audit called it a missing dependency and reported its bundle as
  incomplete. Asking for something by name is the stronger statement, so
  it wins, and the contradiction is reported once with how to clear it.
- A bundle you have switched on no longer installs its items switched off
  because some other, switched-off bundle happens to carry the same item.
  Whichever bundle sorts first used to decide, so the result depended on
  the names. An item two bundles carry is now on if either bundle is on,
  and anything else the two disagree about — which catalog it comes from,
  how it is installed — is reported with both bundle names instead of
  being settled silently.
- Two skills that require each other no longer report each of their
  findings twice in the audit.
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
