# vstack2 Architecture

Cross-platform desktop app (Rust + Tauri) managing AI coding-harness
customizations — agents, skills, hooks, commands, MCP servers, plugins, Pi
extensions — across global and per-project scopes. Claude Code first-class;
codex, opencode, cursor, pi, gemini, and copilot behind the same adapter
seam. No server; a thin CLI mirrors every core operation so consuming-repo
automation (refresh, report, …) keeps working.

## The one idea

Four verbs over one model: **scan → declare → diff → apply**.

- **Scan** — read harness-native directories in place, across all scopes.
  Useful read-only with zero adoption; nothing copies into a shadow store.
- **Declare** — a per-scope `vstack.toml` manifest is the only durable home
  of user intent.
- **Diff** — drift = declared vs observed. The Audit page is this diff.
- **Apply** — make disk match declaration, plan shown first. Adopt is the
  reverse arrow: record an observed item into the manifest.

Every page and every CLI verb is a projection of these four; none owns
logic.

## Vocabulary

Scope (global | project) · Harness (adapter + capability table) · ItemKind
(agent, skill, hook, command, mcp-server, plugin, pi-extension) ·
Item (logical: kind + name from a source) · Installation (item × harness ×
scope — what locks, drift rows, and applies track) · Source (path | git;
registry reserved post-release) · Manifest · Lock (provenance + hash) ·
Observation (scanner truth) · Drift. Core modules mirror the verbs: `model`, `scan`,
`manifest`, `diff`, `apply`, `source`, `harness/` (one file per harness).

## Layout

`crates/core` — pure domain. `crates/app` — Tauri commands, one module per
page domain; events stream scan progress. `crates/cli` — thin verbs over
the same core. `ui/` — React 19 + Tailwind v4 +
shadcn/ui + zustand over generated bindings (tauri-specta). Adapters in
`core/harness/` own paths and rendering only; what each harness supports
lives in one capability table read by core and UI.

## Invariants — what the product guarantees

1. Generated artifacts are always overwritable; refresh regenerates from
   scratch and re-merges the manifest.
2. Write-only-if-absent: never clobber a user-set value; never re-add a
   user removal. This protects manifest values and unrelated
   structured-config keys — managed generated content is replaceable
   (invariant 1); the two never overlap.
3. Content hashes cover source bytes plus the manifest sections that shape
   an artifact — editing a shared key invalidates dependents.
4. Locks record durable provenance; same-source reinstall is a no-op,
   cross-source name collision is a hard error naming the original.
5. Enable/disable is non-destructive and lossless: file-backed kinds
   toggle by rename; kinds embedded in shared config files toggle by a
   structured edit that preserves every unrelated key. Uninstalling the
   app changes nothing.
6. Never touch the unowned: unmanaged files are reported, never deleted;
   foreign symlinks are conflicts, not clobber targets; adoption merges
   content, never loses it. Ownership is what vstack wrote, read from the
   lock — including the paths an installation recorded writing under
   another kind's name. A position we put something at is ours to replace
   or clear, whichever entry holds it now; deriving ownership from the
   lock key alone calls our own output a stranger's.
7. Applies are transactional: preconditions revalidate against observed
   hashes immediately before mutation; pre-images are journaled first; any
   failure rolls back and interrupted applies recover on next launch.
   Removals go to a trash, never straight to delete.
8. One writer per scope: every apply (app or CLI) holds an OS-level scope
   lock; journal recovery runs under the same lock; a busy scope is a
   clear error, never an interleaved write.
9. Never mutate a working tree vstack does not own. Managed scopes are
   the only writable surface; vstack never stages, commits, or resets in
   a repository it did not create. Work that must produce a commit runs
   in a disposable clone, where none of a live tree's states exist.
10. Writes are byte-faithful: a file vstack edits round-trips
    byte-identically except for the intended edit, trailing newline
    included. Change detection compares exact bytes — a comparison that
    ignores trailing whitespace pins the corruption it hides instead of
    letting the next write heal it.
11. Validation precedes mutation. Every input check for an operation
    runs before its first durable write — not merely before the apply
    it guards — and a rejected operation leaves manifest, lock, and
    install tree byte-identical. No failure path leaves persistent
    state changed. Output is checked on the same side of the write:
    every rendering is read back through the target harness's own
    format rules inside plan preview, and one the harness's loader
    would reject is refused there, with the fix, for that harness
    alone.
12. Verification compares content, not provenance. Installed artifacts
    are re-hashed against what they should be; a matching lock entry
    alone never reports OK, and an artifact vstack cannot compare is
    reported as uncompared, never as passing.
13. External processes are hardened by construction. One constructor
    builds every invocation: environment that can redirect it
    (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`) cleared, every
    prompt path closed (`GIT_TERMINAL_PROMPT=0`, SSH `BatchMode=yes`),
    a timeout on every call. Work inside a downloaded cache also pins
    `--git-dir` and `--work-tree` on the command line, which outranks
    config: a cached repository's own `core.worktree` cannot point a
    refresh at files outside its cache. An unhardened invocation is not
    constructible — the raw-`Command` pattern is guard-banned, because
    a per-call-site discipline reliably misses call sites.

## Decisions

- Tauri 2 · React 19 · Vite · Tailwind v4 · shadcn/ui · zustand ·
  tauri-specta · serde/toml.
- Vercel-inspired design language: monochrome, high contrast, minimal
  chrome. Light/dark/system only — no themes. Every color, space, and
  radius flows through design tokens (guard bans raw hex in UI code).
- No database: manifests, locks, and native dirs are the state; scans are
  in-memory views (startup, focus, watch); app prefs in one settings file.
- GUI + CLI are equal thin shells over `crates/core`; every core operation
  has a CLI verb. No CI until first release; `tools/guard` is the gate.
- Multi-harness kept (v1 fleet workflows depend on Pi). Every capability
  ships cross-harness through the capability table; a harness without
  native support for a kind is marked unsupported — never shimmed. Where
  a vendor has itself replaced one surface with another (Codex retired
  its prompt directory in favor of skills), the table names the kind the
  artifact is stored as and the lock records what was written: that is a
  native surface, not a shim.
- **The table says what a verb means, not only whether it exists.**
  Beside op × scope it carries whether a hook the tool loads is executed
  or only read as prose, and the MCP transports the tool speaks.
  `managed` never implied enforcement — a safety hook rendered as
  advisory text must not read as protection, so an advisory install says
  so in the plan preview, the report, and the tool's card. A column only
  earns its place if a verb reads it: what a tool's own configuration
  holds down elsewhere (Copilot's `disabledSkills`, which a repository
  may add to but never take from) is reported per item where it is read,
  because vstack's own switch is a rename it can undo either way and a
  column saying otherwise would forbid a working enable.
- **An adapter claims only its own namespace.** Tools reading each other's
  directories is now common (Copilot reads Claude Code's skills and
  settings). A file belongs to the tool whose namespace it sits in, and
  the cross-read is reported as an input to effective state — never as a
  second installation, which would count one file on disk twice.
- Fresh manifest schema + one-time v1 importer; no compat shims. v1
  extras/theme packs are not carried over.
- **Schemas are versioned and migrations are applies.** The manifest and
  lock carry a format version; older files load, and the upgrade rides
  the normal journaled, previewed plan as a surgical edit (the version
  line changes, nothing else). Files from a newer vstack refuse to load
  — an older build never corrupts a newer file.
- **Permission intent is typed and never widens.** A source's tool
  allowlist survives parse, merge, and every renderer as
  `Unspecified | AllowOnly | DenyExtra`; explicit denies survive
  allowlist subtraction. A surface that cannot express the intent
  renders the most restrictive expressible form or refuses with a
  conflict row — and a refusal also removes the older, wider rendering.
  Converting an allowlist to a deny-list by complement is forbidden: it
  widens the moment the tool grows a new built-in.
- **Catalogs are adversarial input.** Every catalog read goes through
  one sealed API (`source_read`) that resolves against the canonical
  root, refuses symlinks, and carries depth/count/byte budgets; raw
  filesystem reads over catalog paths are guard-banned. Frontmatter
  parses as real YAML under the same posture (aliases and duplicate
  keys refused, bounds enforced), and every interpolated value in a
  generated file is quoted so foreign text cannot mint config lines.
- **The surface model.** Rendered skills are per-harness variants,
  deduplicated by content hash. Harnesses that read the same physical
  directory form a surface group carrying exactly one variant rendered
  to the group's combined constraints (tightest byte cap wins); a
  variant whose bytes match the shared tree collapses onto it through a
  link, and a divergent one gets its own tree. The move runs both ways as
  the source grows and shrinks: a link gives way to a directory and a
  directory back to a link, each planned as a removal plus a write, since
  a variant left reading a stale link gets exactly the truncation the
  split exists to prevent. A refusal is per surface, not per tool — the
  members of a group all read one file — and it takes down only what the
  refusing installation alone holds. Format facts — byte caps, name
  rules — live in one table beside the op table (`harness/caps.rs`),
  never as renderer literals. A surface is one file per item, one
  directory per item, one structured file, or a directory of structured
  documents (Copilot loads every `*.json` in its hooks directory as a
  document of its own): where the entries inside are the items, a
  document holding none reports none, so an emptied registration cannot
  read as a live installation.
- One model-alias table for every harness: bare tiers resolve per
  harness, `inherit` is expressed in each tool's own dialect, explicit
  vendor ids pass through.
- **Propagation into consuming repos is local, never a pull request.**
  vstack detects drift and informs the agent at session start; the repo
  is brought current by a local refresh. Opening PRs in consumer repos
  is a permanent non-goal: the managed assets are gitignored there, so
  there is nothing to commit, and the attempt would mean mutating a
  live foreign working tree (invariant 9).
- Non-interactive is a mode, not a fallback. Every CLI verb completes
  without a TTY: selection flags suppress prompts rather than
  pre-filling them, and a verb that would need input on a non-TTY fails
  before its first write, naming the flag that answers it. Agent- and
  CI-driven runs are the normal case. Interactive selection lives in
  the GUI; the CLI has no pickers.
- vstack never emits a pasteable command line. Errors, hints, and
  recovery instructions present the verb and its parameters as data —
  cross-platform shell quoting is a cost the product declines to carry,
  and a hint built by concatenation is an injection surface.

Build sequence and to-build specs: `docs/PLAN.md` — consumed and deleted as
phases complete; delete this pointer with that file.
