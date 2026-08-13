# vstack2 v0.2 — the best-of-breed cycle

> **Cycle progress** (updated every commit; deleted sections = landed):
>
> - Phase 0 ✅ (commit 221f950) — changelog, app-deploy skill, commit-msg
>   changelog rule.
> - Phase 1 ⏳ in flight. Landed: bug 1 (typed `PermissionIntent` through
>   all five renderers, optional role, Codex sandbox inference, Claude
>   native allowlist, OpenCode permission synthesis incl. MCP-only lists,
>   Pi refusal → conflict + removal), bounded tolerant YAML frontmatter
>   parser, YAML/TOML output quoting (injection-proof), `allow-tools`
>   override, v1 import keeps legacy `tools` incl. harness-agnostic
>   tables; adversarially reviewed (14 findings fixed). Bug 2 fixed: one
>   model-alias table (`harness/models.rs`) wired through all renderers;
>   `model_id_for`/`codex_model` deleted. Next in sequence: sealed source
>   reads (bug 3), coalesced config edits (bug 4),
>   scope canonicalization, schema v2 + migration, format caps + warnings
>   channel, body-split + tool swap + commands→skills + name legality,
>   validators, hardened process constructor, byte-faithful writes,
>   content verification.

This file is consumed: delete items as they land, delete the file when the
cycle ends (with the `docs/research/` reports and the ARCHITECTURE.md
pointer to this file). Every phase ends with a working app and a fully
green suite (`tools/guard` passes). Research ran first and is complete;
the UI pass runs last; the cycle closes with a `v0.2.0` draft release per
`docs/RELEASING.md`.

Standing rules for every phase:

- Structural reshapes amend `docs/ARCHITECTURE.md` in the same change
  (the AGENTS.md rule). Durable decisions born in this plan — the
  capability model, the surface model, provenance edges, the source
  store, quality gates — must live in ARCHITECTURE.md before Phase 7
  deletes this file. Durable *facts* from the research (the two
  observation matrices' locations, formats, precedence rules, official
  doc links) move into per-adapter docs before the research is deleted —
  future capability changes need their evidence reviewable.
- Adversarial review before merge for all engine/render work (see
  "Parallel work" below).

The cycle in one sentence: adopt wshobson/agents' emission mechanics and
quality gates where they beat ours, add Gemini CLI and GitHub Copilot as
tools six and seven, make catalogs able to consume marketplace-shaped
repos safely, ship bundles and real dependencies on a sound provenance
model, then repolish the entire UI.

## Research (complete)

| Report | Feeds |
|---|---|
| `docs/research/wshobson-agents.md` | decision table 1, Phases 1 & 3 |
| `docs/research/gemini-copilot-matrix.md` | Phase 2 (verified against official docs 2026-08-10) |
| `docs/research/harnesskit.md` | decision table 2, Phase 5 |
| `docs/research/v1-baseline.md` | Phases 3 & 4 (dependencies, lock, changelog) |
| `docs/research/skillshare.md` | Phase 4 (provenance, uninstall) |

Two rounds of external cross-model review (Codex, adversarial)
stress-tested this plan; accepted findings are folded in — most visibly
the surface model, the typed permission intent, per-harness rendered
trees, the intent-vs-cache provenance split, centralized source
containment, coalesced config edits, the immutable source store, the
capability-model extension, and the Critical-blocks-independently gate.

Live bugs the research surfaced — all fixed in Phase 1, first:

1. **Privilege escalation on import** — `parse_source_agent`
   (`render/agent/mod.rs:83`) silently drops `tools:`, and a missing
   `role:` defaults to Engineer, which `render/agent/codex.rs:45` renders
   as `sandbox_mode = "danger-full-access"`. A read-only reviewer from any
   foreign catalog becomes a full-access agent. The v1 importer has the
   same hole: `import_v1/mod.rs:156` *deliberately drops* legacy `tools:`
   allowlists, so a restricted v1 agent migrates unrestricted.
2. **`model: inherit` breaks OpenCode** — `model_id_for("openai",
   "inherit")` emits `"openai/inherit"`, an invalid model id, and
   `inherit` is exactly what portable catalogs use.
3. **Foreign catalogs can escape their source tree** — catalog reads
   follow symlinks (`render/skill.rs` `collect` via `is_dir`/`fs::read`,
   `source.rs:191` `find_item` via `is_file`), so a hostile catalog can
   pull host files into generated artifacts or recurse forever.
4. **Two structured edits to one config file cannot both apply** — each
   registration plans an `EditFile` against the same original hash
   (`engine/item_plan.rs:335`); the second precondition fails and the
   transaction rolls back. Installing two MCP servers into one
   `settings.json` in one apply is impossible today — bundles would make
   this the common case.

## Decision table 1 — render & adapter mechanisms (directive 1)

Verdicts against wshobson/agents @ `c4b82b0`; evidence and file refs in
`docs/research/wshobson-agents.md` §2.

| Mechanism | Verdict | Rationale |
|---|---|---|
| Capability matrix drives every adapter | **hybrid** | Keep our op table and its honesty tests (they have none); add a sibling *format* table (byte caps, tool-name case, model dialect, MCP transport) so caps own format facts, not renderer literals. Phase 2 extends the model further — see "capability model v2". |
| Tool-vocabulary remapping | **hybrid** | Adopt the identifier swap for structured fields *and* their conservative prose rewrite (shared pattern between rewriter and lint, rewrite visible in plan preview; code fences, inline literals, links, and generated path references are never rewritten) — deep idiomatic output is binding. Our tables come from official docs (6 of 8 of their Gemini mappings are wrong). Custom/MCP/unknown identifiers pass through with a warning; entries are never dropped from deny-lists (dropping widens access). |
| Model-alias mapping across vendors | **adopt** | One table keyed `(harness, alias)`, tier-preserving (incl. a `fable` tier), warning on unknown alias; only bare aliases map — explicit vendor ids pass through untouched. Replaces our two disagreeing ad-hoc functions and fixes bug 2. |
| Body-size caps + overflow into `references/` | **adopt** | We have no cap at all — a 40 KB skill silently truncates on Codex today; their fence-aware split and UTF-8-safe cut port as written. Split invariants: the cap check runs *after* instruction injection and accounts for the pointer note it appends; injected project instructions are never moved into references (they must stay authoritative); a single fenced block larger than the cap is a finding, never a mid-fence cut; overflow filenames have collision rules. |
| Commands → skills where no command surface exists | **hybrid** | Codex only, where the vendor itself deprecated prompts in favor of skills (native surface, not a shim); take the collision-suffix logic. The lock records the logical→emitted mapping (emitted kind, name, paths) so removal and refresh target what was actually written. OpenCode/Cursor stay observe-only per the "never shimmed" rule. |
| Permission/allowlist synthesis from `tools:` | **hybrid** | Permission intent becomes a typed, lossless value — `Unspecified | AllowOnly(list) | DenyExtra(list)` — preserved from source to renderer. A deny-only surface cannot express `AllowOnly`; converting by complement widens access the moment the tool grows a new built-in, so an adapter that cannot express the intent renders the most restrictive expressible form or refuses with a finding — it never widens. Adopt their missing-vs-empty distinction, OpenCode permission-block synthesis, and Codex read-only sandbox inference — this fixes bug 1. |
| Namespacing + per-harness name legality | **adopt** | Required the moment a 91-plugin catalog lands; names like `My_Skill` currently render to OpenCode and fail its loader with no warning from us. |
| Per-harness frontmatter stripping | **keep** | Our renderers build frontmatter from scratch; nothing to strip. |
| YAML scalar quoting discipline | **adopt** | ~20 lines; a description of `no` currently type-coerces in OpenCode output. |
| Warnings channel with fix strings | **adopt** | Extend `DesiredState.notes` into per-item render warnings with remediation strings, surfaced in plan preview and CLI — the delivery vehicle for every other lint here. |
| Orphan pruning | **keep** | Lock-provenance sweep survives a broken source; their set-difference prune doesn't. |
| Context-file budget management | **keep out** | Context files aren't an ItemKind; see open question 2 before this changes. |
| Large-body file injection | **keep** | We have the pattern (`skills_prose`); reuse Gemini's native `@{path}` syntax in the new adapter where it applies. |

## Decision table 2 — quality gates & scoring (directive 7)

Verdicts against HarnessKit @ `461a7a1` and wshobson's validation stack;
evidence in `docs/research/harnesskit.md`.

| Piece | Verdict | Rationale |
|---|---|---|
| Rules-engine shape (trait + registry) | **adopt** (HarnessKit) | One `AuditRule` trait, one registry; add a `remediation` field to findings, drop the trait-level severity (three of their rules already contradict it). Inputs are *typed per kind* (skill tree with byte/file budgets, hook registration, MCP command+args+env+headers, plugin manifest+scripts) — "content" is defined per kind, not assumed. |
| Safety rules (6 content, 3 MCP/permission, 2 plugin) | **adopt, amended** | Port the 11 rules; replace the fenced-code exemption (a real bypass) with "fenced content scans at one severity lower — except secret/token matches, which never downgrade". Secret findings are redacted: the matched token never appears in messages, logs, or UI. Plugin rules run where plugin sources are actually readable (installed files at scan; resolved content at install) and are explicitly n/a elsewhere — a desired plugin is only a settings edit today. |
| Their 5 `cli-*` rules | **skip** | vstack has no installed-CLI ItemKind. |
| Safety scoring math | **adopt** (HarnessKit) | `100 − Σ deductions` (25/15/8/3), first hit full, repeats −1, floor 0 — every deduction names a rule at a location. The aggregate score *warns*; blocking is per-finding (next row), because threshold math alone lets a single Critical (score 75) sail through. |
| Blocking rule | **new** | Any Critical finding blocks on its own, independent of the aggregate; aggregate warn < 80, block < 60. Overrides bind to the exact decision: (installation, rendered-content hash, ruleset version, finding fingerprints) — stale the moment any of them changes, recorded in the manifest *in the same transaction as the apply they unblock*, visible in Audit. A one-time review must never become a standing bypass. |
| Deobfuscation | **hybrid** | Their invisible-char strip + the parked vstack ideas: NFKC + homoglyph folding, with normalization-that-changes-content reported as a finding (severity calibrated against real catalogs during the phase — legitimate text also normalizes). |
| Quality scoring | **adopt static layer only** (wshobson) | Weighted dimensions + multiplicative anti-pattern penalty, advisory, never blocking; skip the LLM judge, Monte Carlo, Elo, badges, letter grades — wrong cost model for a desktop install path. |
| Structural output validators | **adopt, relocated** | Their `validate_generated.py` checks become Rust validators living beside each adapter, run *inside plan preview* so errors block apply — strictly stronger than their after-the-fact CI. |
| Drift detection | **keep** | Our hash-vs-observation Audit beats their mtime gardener. |
| Fix strings on every finding | **adopt** | The single highest-value idea in either source. |
| Real-CLI round-trip smoke tests | **adopt, re-aimed** | Test *our* surfaces, not theirs: stage the exact personal/project trees vstack emits and run each CLI's local parse/list against them, for all seven adapters (Copilot included). Opt-in dev target + CI, where at least one job installs the freely installable CLIs — a run where every check skipped is a failure to gate, reported as such, never green. |
| Gate placement | **hybrid, one novel piece** | Authoring checks in `vstack check`/`init`. Safety scoring runs in *two* places: on the desired rendered artifact at plan time (that's what gates a fresh install — an uninstalled item has no observation), and on observed content at scan (that's what the Audit page shows). Blocking happens at apply. |
| Score presentation | **adopt** (HarnessKit) | Safety and quality are two scores, never averaged; Audit rows gain a findings column, no new page. |

## Phases

### Phase 1 — render pipeline v2 + engine foundations (directives 1 + 2)

Implement every **adopt**/**hybrid** row of decision table 1, plus the
foundations the external reviews showed are prerequisites:

1. Fix the four bugs (regression tests first): a `tools:`-restricted,
   role-less agent must not render `danger-full-access` — in the source
   parser *and* in `import_v1` (v1 allowlists become `AllowOnly` intent,
   never dropped); `inherit` must survive every harness; catalog reads
   must be contained; shared-config edits must coalesce (items 3 and 6).
2. Replace the hand-rolled frontmatter parser with real YAML parsing
   into typed per-kind source models (dependency justified in the
   commit): preserves missing-vs-empty (the permission model depends on
   it), handles block scalars, arrays, nested maps; unknown keys warn
   instead of vanishing. Parsing is *bounded* — alias/depth/size/node
   limits, duplicate security-relevant keys rejected — foreign YAML is
   adversarial input, not just untidy input.
3. **One sealed source-read API.** Every catalog read — items, skill
   trees, source config, registries, plugin manifests — goes through one
   API that resolves against the canonical source root, refuses symlinks
   in foreign sources (`symlink_metadata`, loud finding), and enforces
   depth/count/byte budgets. Path-local `is_dir`/`is_file`/`fs::read`
   over catalog paths becomes a guard-able pattern to ban.
4. **The surface model + per-harness rendered trees.** Today
   `skill_canonical` renders one shared tree for all harnesses
   (`engine/desired.rs:97`) — per-tool output is impossible, and the
   deeper truth is that *physical surfaces are shared*: Codex and Pi
   both consume `.agents/skills` (`harness/codex.rs:50`,
   `harness/pi.rs:51`), and Copilot reads `.claude/skills` besides its
   own dirs. So the model is: rendered artifacts are per-harness,
   deduplicated by content hash — but a physical surface consumed by
   several harnesses (a *surface group*) carries exactly one variant,
   rendered to the group's combined constraints (tightest byte cap,
   intersection of format features). If group members' requirements are
   genuinely incompatible, that install is a finding, not a silent
   pick-one. Cross-reads outside the group (Copilot seeing a Claude
   variant) are surfaced in Audit as duplicate effective definitions.
   Generated paths move: journaled apply migrates installs (B9).
5. **Coalesced config edits.** The plan composes all edits to one
   config file into a single deterministic mutation with a single
   precondition (a mutation graph keyed by canonical path) — two hooks,
   three MCP servers, and a plugin toggle in one apply become one write
   per file. Prerequisite for bundles and for Gemini/Copilot settings
   management.
6. Scope identity is canonicalized inside core before planning and
   applying — the scope lock must not depend on callers normalizing
   paths (`apply/mod.rs:14` hashes path text today).
7. **Manifest/lock schema v2 lands here**, versioned load + a
   transactional, journaled v0.1→v0.2 migration with fixtures — before
   the first schema-dependent change ships, not after (Phases 2–5 extend
   the schema; "loads with defaults" is not a migration story).
8. The format capability table beside the op table in `caps.rs`; the
   model-alias table (delete `model_id_for` and `codex_model`); the
   render-warnings channel with remediation strings through plan preview
   and CLI.
9. Body-size cap + `references/` overflow (split invariants per decision
   table); tool-name swap + prose rewrite; typed permission synthesis;
   commands→skills on Codex with lock-recorded emitted mapping; name
   legality; YAML quoting.
10. Per-adapter structural validators — they co-land here because they
    assert exactly what the renderers emit; the rules engine itself
    waits for Phase 5.
11. **One hardened process constructor** (invariant 13). Every external
    invocation is built by it: redirect vars cleared, prompt paths
    closed, timeout set; `Command::new` outside it is guard-banned.
    Prerequisite for Phase 3's source store — v1 has 27 unguarded
    `Command::new("git")` sites and three `git reset --hard`, so a
    cache repo whose `.git/config` sets `core.worktree` elsewhere (or
    an inherited `GIT_WORK_TREE`) resets files outside the cache.
12. **Byte-faithful writes + validate-before-mutate** (invariants 10
    and 11). A round-trip test per writer proves the intended edit is
    the only byte difference; a rejection test proves manifest, lock,
    and install tree are byte-identical after a refused operation. No
    "compare ignoring trailing newline" helper is written: v1 needs
    `project_config.rs::same_ignoring_trailing_newline` only because two
    of its section writers drop the trailing newline that twelve
    siblings restore, and that comparison then pins the malformed output
    as unchanged forever.
13. **Verification compares content** (invariant 12): drift rows re-hash
    installed artifacts, and an artifact that cannot be compared is
    reported uncompared. v1's `verify` checks path existence for skills
    and agents and counts an uncomparable install as OK
    (`commands/verify.rs::verify_entry`), so a Codex agent widened to
    `danger-full-access` after install still reports `install:✓`.

**Done when:** decision-table rows checked off with a failing-first test
each; an oversized skill produces a split, not a truncation; a foreign
catalog agent imports with its restrictions intact (source parse and v1
import both) and an inexpressible restriction refuses rather than
widens; a symlinked catalog cannot read outside its root through *any*
read path; a git operation cannot escape its cache directory through
`core.worktree` or an inherited `GIT_WORK_TREE`, and cannot block on a
credential prompt; every config writer round-trips byte-identically and
a refused operation mutates nothing; a tampered install fails
verification; a Codex+Pi project install shares one `.agents/skills`
variant rendered to group constraints; two MCP servers install into one
settings file in one apply; v0.1 fixtures migrate; render warnings show
in plan preview; suite green.

### Phase 2 — Gemini CLI + GitHub Copilot (directive 3)

Follow the proven pattern: observation first, then capability-gated
management. The observation matrix is already written and doc-verified
(`docs/research/gemini-copilot-matrix.md`); its §7 table is the caps
seed.

**Capability model v2 comes first.** Op × scope booleans cannot state
the truth about these tools: Copilot project scope can disable but not
enable skills/MCP (directional merges), hooks are manageable file-backed
but observe-only inline, Gemini MCP servers declare per-project but
toggle in a global file, Gemini agents sit behind an experimental flag,
MCP transports differ per harness (Codex takes Streamable HTTP, not
SSE). The table gains the axes to say so — toggle direction, surface
subtype, transport, feature-flag dependency — generated into the UI
bindings like everything else, and the honesty tests extend to the new
axes. Claiming `managed(BOTH)` where only half the verbs work would
break "the capability table gates everything".

**Enforcement is one of those axes.** `managed` today says vstack can
write and track an artifact; it does not say the harness will run it.
Both are `managed(BOTH)` for Hook, yet Claude registers an executable
whose exit code gates the tool call, while Cursor gets a `.mdc` rule
with no registration (`engine/targets.rs`), OpenCode gets instruction
files plus config refs (`caps.rs`), and Codex is native only for the
events `hook.rs` maps and advisory prose otherwise. Gemini and Copilot
both arrive with real hook systems (research §D1, §D9), which makes the
gap wider, not narrower. So caps gain an **enforcement level** —
enforced (the harness runs the command and honors its result) vs
advisory (rendered as text the model may ignore) — resolved per
(harness, kind, event), carried into plan preview, Audit, and the item
UI. A safety hook must never read as protection on a harness that can
only suggest it. `(Pi, Hook) => unsupported()` is the model working
correctly and stays as is.

Adapter facts that shape the code:

- Copilot's global root honors `$COPILOT_HOME`; resolve through `Env`.
- Each adapter claims only its own namespace — Copilot claims
  `.github/**` + `~/.copilot/**`, Gemini claims `.gemini/**` +
  `~/.gemini/**`. Copilot genuinely reads `.claude/` files; those
  cross-reads are *inputs to Copilot's effective state* (a
  `disableAllHooks` in `.claude/settings.json` disables Copilot's hooks
  too) and Audit reports duplicate effective definitions — but never a
  second installation (no double-counting).
- Copilot commands are **unsupported** (no such surface). Gemini
  extensions: observe global-only (undocumented enablement file).
- Gemini MCP *enablement* is modeled global-only (its state file is
  global) — a project-scope toggle writing a global file under a project
  lock would break one-writer-per-scope. Project-scope declaration
  install/remove stays project-scoped.
- **Effective-state scanning, honestly bounded:** read the observable
  layers that can defeat a write — Gemini system-override settings,
  `experimental.enableAgents`, Copilot's repo-scope key allowlist,
  folder trust, `.github/allowed_models.txt`, the `.claude` cross-reads
  — and let Audit say "present but inert (overridden)". Invocation-time
  overrides (env vars, CLI flags) are unobservable; Audit language
  stays "as configured", never claiming runtime behavior.
- Both tools' configs migrated recently: scanners tolerate old and new
  shapes on read, write only the current one — and when a machine has
  *only* the old shape (an un-upgraded CLI), management for that surface
  is reported unsupported rather than writing files the installed tool
  won't read.
- Plugin declarations gain a harness target before any Copilot plugin
  toggle lands — today `PluginDecl` broadcasts to every harness, which
  would write Copilot plugin keys into Claude settings (register B11).
- Models: Gemini tiers map to `gemini-3-*-preview` with `inherit`
  preferred when unset; Copilot gets **no hardcoded tier map** — emit
  `auto` or pass user strings through (plan/org/allowlist-gated).
- Two new readers (Copilot hook JSON, `enabledPlugins` map).
- README support matrix + `labels.ts` gain both tools.

**Done when:** detection roots proven against fixtures for both tools and
both scopes; every §7 caps row implemented with the v2 axes and covered
by the extended honesty tests; every hook-capable harness carries an
enforcement level that the UI and plan preview show, with an advisory
install stating plainly that it is not enforced; managed kinds round-trip
install/toggle/remove; an inert-but-present installation is reported as
such; renderers produce doc-valid output (Phase 1 validators assert it);
suite green.

### Phase 3 — catalog layout v2 + marketplace consumption (directive 5)

The current v1-shaped catalog (kind dirs at the root) stays supported.
Added: a wshobson-shaped repo (`plugins/<name>/{agents,commands,skills}`
+ registry) is consumable directly as a catalog — safely.

- **Recognition is explicit, not heuristic:** a source is
  marketplace-shaped iff it carries the recognized registry file
  (`.claude-plugin/marketplace.json`) that parses and validates.
  Validation is cross-file: component paths must resolve beneath their
  plugin root (through the sealed source API), duplicate plugin
  identities, registry↔plugin.json name/version disagreement, and
  Unicode/case-fold/trailing-dot filename collisions are findings. Only
  local-path entries are consumed in v0.2; `git-subdir` and URL entries
  are skipped with a finding naming the entry (4/95 in the reference
  repo). No guessing from a `plugins/` directory.
- Namespacing: items from marketplace-shaped catalogs are named
  `<plugin>/<leaf>` in manifest and UI; per-harness rendering maps to
  each tool's legal separator (`__` where `/` is illegal), with
  collision rules (case-fold collisions, `a/b` vs a literal `a__b`,
  path-hostile names: `..`, device names, overlong components — all
  findings, never silent). Flat names from v1-shaped catalogs are
  unchanged.
- **The source store becomes immutable and pinned-safe.** Today one
  mutable checkout per repo is hard-reset on refresh (`remote.rs`) —
  two scopes pinning different revisions would fight over it, and a
  refresh mid-plan can shift bytes under a render. Replace with
  per-commit checkouts keyed by (repo identity, full commit OID),
  published atomically, verified unmodified on reuse, under a
  repository-level cache lock. **Pin semantics, decided:** a *commit* is
  a pin — durable intent, recorded in the manifest. A *tag or branch*
  is a tracking selector — it re-resolves on refresh, and the lock
  records the resolved commit as reproducibility cache (losing the lock
  loses no intent either way; a moved tag is followed on the next
  refresh, previewed like any upstream change). Offline with an
  uncached pin: hard error; existing installs keep working from
  rendered state.
- Catalog-item metadata (category, version, author, license, homepage)
  gets a read-side struct feeding Phase 4's browse UI — deliberately not
  part of the manifest. A dedicated store-like registry experience
  (multiple browsable marketplaces, Vercel skills marketplace,
  add-your-own registries) is out of scope for v0.2 by owner decision —
  recorded in `docs/roadmaps/future.md`.
- A wshobson "plugin" maps to a vstack **bundle** (Phase 4) — the group
  identity is preserved at parse time so Phase 4 can install it as a
  unit; nothing in this phase installs groups yet.

**Done when:** the default catalog and a pinned wshobson clone both
enumerate, install, and refresh correctly; two scopes pinning different
revisions of one repo coexist (test); a hostile catalog fixture
(symlink escape anywhere, path-hostile names, oversized tree,
cross-file registry lies) is rejected with findings; a moved tag
re-resolves preview-first; breaking changes registered; suite green.

### Phase 4 — bundles + dependencies (directive 4)

Both concepts, kept distinct: a **bundle** is a curated installable set
(agents + skills + commands + hooks) with a name and provenance; a
**dependency** is one item requiring another.

**The provenance model — intent in the manifest, cache in the lock.**
The manifest (the only durable home of intent) records *choices, not
closure*: the requested-item set, installed bundles, optional-dependency
selections, and suppressions (a member or dependency the user removed
and wants kept removed). Derived members and dependencies are **not**
written as individual declarations — that would promote every derived
install into a user request and break uninstall. The plan derives the
closure; the lock caches it as **reason edges** — structured values
(edge kind + the source/kind/name/harness/scope of the counterpart),
not strings: `requested`, `required-by`, `member-of`. An installation
holds a *set* of edges (requested *and* member of two bundles *and*
required by three items); losing the lock loses nothing — the graph
rebuilds from manifest + catalogs.

**Dependencies** — keep v1's frontmatter schema (`dependencies:
{required, optional}`, bare names; minimal and sound), fix the semantics
it botched, and scope it honestly:

- Skills-only, same-source-only in v0.2, as in v1 (a catalog author
  cannot know a consumer's source aliases, so cross-source references
  have no stable identity to name — cross-kind and cross-source
  curation is what bundles are for). Unresolvable or ambiguous names
  are findings, never silent drops.
- Harness propagation: a dependency installs for the parent's harness
  set intersected with its own support; the missing remainder is a
  warning on the parent, not a block.
- No body-scrape fallback (v1's 106-line prose parser dies here).
- Cycles are detected and reported as info — v1's real `orch ↔ dev`
  pair is an intentional co-install, not an error; only an unresolvable
  graph blocks. Suppressing a member of a required cycle marks every
  parent in the cycle with the missing-dependency warning.
- Refresh re-expands, preview-first, in *both* directions: upstream
  additions **and** removals (a bundle dropping a member, a dep
  disappearing) show in the plan; nothing installs or uninstalls
  without the preview/confirm step. This changes CLI `refresh`, which
  today applies its plan directly (`refresh.rs:58`) — regeneration of
  existing installations stays automatic, set changes require
  confirmation or `--yes` (register B10). On a non-TTY the confirm step
  refuses before writing anything rather than prompting — the standing
  non-interactive rule, which every new prompt in this cycle inherits.
- A user's removal of a required dep records a suppression; refresh
  honors it; Audit shows a "missing required dependency" warning on the
  parent rather than resurrecting the dep (durable-removal semantics,
  same shape as `mapping.rs`).
- Optional deps become a real install-time choice (checkbox in UI, flag
  in CLI), persisted as manifest selections — the choice survives
  refresh and other machines.
- Remove is dependency-aware: removing an item warns about dependents;
  removing the last dependent offers to sweep orphans whose only
  remaining edge was `required-by` it.
- Removal ops bind to real preconditions (today `removal.rs` trashes
  with `Pre::Any` — a file changed after preview still gets moved):
  hash for files/trees, link target for managed symlinks. Bundle
  uninstall multiplies the stakes.

**Bundles:**

- Catalog authoring: v1-shaped catalogs declare bundles in the source's
  `vstack.toml` (`[bundles.<name>]` with member lists + description);
  marketplace-shaped catalogs get them for free — each plugin entry *is*
  a bundle (manifest, members, version, category from Phase 3).
- Manifest declaration: `[bundles]` in the user's `vstack.toml` declares
  the installed bundle; scan/diff/apply stay item-level over the derived
  members (no new verbs).
- Uninstall semantics — **decided**: removing a bundle removes the
  members whose only remaining reason edges came from that bundle;
  members that are also `requested`, `required-by` a surviving item, or
  `member-of` another installed bundle stay. The preview lists exactly
  what goes and what stays, with the reason. Justification: the edge
  set makes this the only answer that never deletes something the user
  asked for and never strands unwanted leftovers — the two failure
  modes v1's model couldn't even express.
- Removing a single member from an installed bundle records a
  suppression (same mechanism as dependencies): refresh neither
  resurrects it nor pretends the bundle is complete — Audit shows the
  bundle as "installed, N members held back".
- UI: bundles browsable/installable from the Sources page (metadata from
  Phase 3); the full Library/Catalogs reshape waits for Phase 6.

**Done when:** bundle install/uninstall round-trips with edge-tracked
members and the preview shows the keep/remove split with reasons; a
multi-edge installation (requested + bundle member + dependency) behaves
correctly through every removal order; suppressions survive refresh and
lock loss; an upstream member removal previews before anything
uninstalls; dangling deps and cycles are findings; optional selections
persist in the manifest; removal preconditions bind to hashes/targets;
suite green.

### Phase 5 — quality gates + scoring (directive 7)

Implement decision table 2 (the structural validators already landed in
Phase 1):

- `core/src/quality/`: rules engine (trait + registry + deobfuscation +
  scoring) over typed per-kind inputs with byte/file budgets, the 11
  safety rules, the static quality dimensions. Findings carry
  `remediation`; secret matches are redacted everywhere.
- Two scoring paths: desired rendered artifacts at plan time (gates
  installs — an uninstalled item has no observation), observed content
  at scan (feeds Audit). Same rules, two defined inputs; rules that
  need bytes a path doesn't have (plugin sources pre-install) are
  explicitly n/a there, not silently passing.
- Surfacing: Audit rows gain findings + safety score; plan preview shows
  findings before apply; `vstack check` runs authoring validation on a
  catalog; `vstack init` scaffolds pass clean.
- Gating at apply: any Critical finding blocks on its own; aggregate
  warns below 80, blocks below 60. Overrides bind to (installation,
  rendered-content hash, ruleset version, finding fingerprints), are
  recorded in the manifest in the same transaction as the apply they
  unblock, go stale on any change, and are visible in Audit.
- CI: ship the `vstack check` gate and a reusable workflow template in
  this repo. Wiring it into the default-catalog repo
  (`vanillagreencom/vstack`) is coordinated with the owner as a separate
  task — its CI is not this repo's acceptance. The smoke suite stages
  the exact trees vstack emits and runs each CLI's local parse/list
  against them, all seven adapters; at least one CI job installs the
  freely installable CLIs, and a run where every check skipped is a
  reported failure to gate, never green.

**Done when:** rules ported with per-rule tests (fence downgrade with
the secrets exception, obfuscation finding, redaction); scores visible
in Audit and plan; Critical-blocks + fully-bound override round-trips
and goes stale on a ruleset bump; `vstack check` exits non-zero on a
seeded-bad catalog fixture; suite green.

### Phase 6 — UI/UX repolish (directive 8, LAST)

The owner finds the current experience disappointing: categories, layout,
and information architecture are not intuitive. This pass may restructure
navigation and page taxonomy — bundles will likely reshape
Library/Catalogs — under the standing rules: plain-language vocabulary
(`labels.ts` is the gate), exactly one primary action per view,
confirm-with-preview before any file change.

Method, mirroring what worked last cycle:

- Cheap models (Sonnet-class or below) for UI implementation and
  validation; the main loop reviews, it doesn't pixel-push.
- Iterate by driving the real pages in Chromium:
  `VITE_MOCK=1 npx vite --port 5273` + agent-browser, screenshots
  archived per iteration *outside the repo* (session artifacts — the
  guard's 200 KB ceiling and the repo both stay clean); iterate until it
  genuinely reads as a polished consumer product, not until it merely
  passes.
- The mock loop exercises pages, not the product: before sign-off, a
  real-app (Tauri) walkthrough covers the flows mocks can't — preview →
  apply → rollback, migration, scope-busy and error states.
- An explicit IA proposal (nav structure, page inventory, where bundles
  live) goes to the owner **before** implementation — that's the one
  approval gate in this phase.

**Done when:** every page walked in the browser with before/after
screenshots; vocabulary audit passes (no eng-speak anywhere
user-visible); one-primary-action check passes per view; the real-app
walkthrough passes; owner walkthrough sign-off; suite green including UI
tests.

### Phase 7 — release v0.2.0

Per `docs/RELEASING.md` and the Phase 0 skill: confirm ARCHITECTURE.md
and the per-adapter docs carry every durable decision and fact, finalize
the changelog from the register below, bump versions, tag `v0.2.0`,
review the draft. Delete this file, `docs/research/`, and the
ARCHITECTURE.md pointer in the closing commit.

**Done when:** draft release exists with CLI binaries, bundles, and
`feed.json`; CHANGELOG `0.2.0` entry complete with every breaking change;
this file is gone.

## Breaking-changes register

Feeds CHANGELOG.md — every row lands there with a **Breaking** call-out
and its migration note. Real v1 repos cross via `vstack import`; v0.1
repos cross via the Phase 1 schema migration.

| # | Change | Phase | Migration |
|---|---|---|---|
| B1 | Missing `role:`/`tools:` no longer renders Codex `danger-full-access`; permission intent is typed and never widened — source parse *and* v1 import | 1 | refresh regenerates; import carries restrictions; agents that *want* full access declare it explicitly |
| B2 | Model aliases resolve through one table; `openai/inherit` and tier-collapsing outputs change; explicit ids pass through | 1 | refresh regenerates |
| B3 | Oversized Codex skills split into `references/` instead of truncating | 1 | refresh regenerates |
| B4 | Commands render to Codex as skills (new artifacts, `__command` collision suffixes, emitted mapping in lock) | 1 | refresh creates them; none existed before |
| B5 | Manifest + lock schema v2 (versioned load + journaled v0.1→v0.2 migration lands with the first schema change): reason edges, suppressions, optional selections, source selectors + resolved commits, overrides, harness-targeted plugins | 1–5 | explicit transactional migration with fixtures; `vstack import` still covers v1 |
| B6 | Lock provenance: reason-edge set on every installation | 4 | migration backfills a single `requested` edge (the only safe reading) |
| B7 | Items from marketplace-shaped catalogs are namespaced `<plugin>/<leaf>`, with collision/sanitization rules | 3 | new sources only; flat names from v1-shaped catalogs unchanged; collisions with existing names are hard errors naming both |
| B8 | Apply can block: any Critical safety finding, or aggregate score below threshold | 5 | fully-bound override recorded per item; thresholds configurable |
| B9 | Rendered artifacts become per-harness trees (hash-deduplicated) under the surface model; generated paths move | 1 | journaled apply migrates installs; refresh regenerates |
| B10 | CLI `refresh` no longer changes the installed set silently (additions *or* removals) — regeneration stays automatic, set changes need confirm/`--yes` | 4 | flag documented; scripted callers add `--yes` |
| B11 | Plugin declarations target a harness (no more broadcast to every tool) | 2 | migration assigns the harness that actually has the plugin |
| B12 | Source cache moves to immutable per-commit checkouts; commit = pin, tag/branch = tracking selector | 3 | cache rebuilds itself; no user data involved |

## Parallel work, review, and models

- **Disjoint ownership per fan-out.** Phase 2 splits cleanly: one agent
  owns `harness/gemini.rs` + `render/agent/gemini.rs` + its scan/tests,
  another owns the Copilot mirror set — after the shared
  enum/caps/model edits land in one commit from the main loop first.
  Phase 5's `core/src/quality/` is disjoint from everything. Phase 1 is
  sequenced, not fanned out — its items share `render/` and `engine/`.
- **Adversarial review is mandatory for engine/render changes** (Phases
  1–5 core work): a second agent, prompted to break invariants 1–8 with
  failing tests, reviews every such change before it merges. UI-only
  changes skip it.
- **Cheap models for UI work** (Phase 6, and UI touches in 2/4/5):
  implementation and browser validation on Sonnet-class or below; core
  engine work stays on the strongest model.
- Session quirks (subagent report recovery, WebKit-on-Wayland env) are
  documented in the user-level CLAUDE.md and `crates/app/src/lib.rs`.

## Owner decisions (resolved 2026-08-10; propagation 2026-08-12)

- **Marketplace shopping stays lightweight in v0.2** — install from
  marketplace repos via the existing pages; the store-like browsing
  experience moved to `docs/roadmaps/future.md`.
- **Default catalog keeps its current layout** and gains a `[bundles]`
  declaration — no restructure, zero migration risk (Phase 3/4).
- **Instruction files stay out of v0.2 entirely** (`CLAUDE.md`,
  `GEMINI.md`, `copilot-instructions.md`): observe nothing, manage
  nothing; revisit as its own cycle (`docs/roadmaps/future.md`).
- **Safety gate: warn below 80, block below 60** on the aggregate score,
  Critical findings always block, fully-bound override available
  (Phase 5).
- **Auto-PR propagation into consuming repos is a permanent non-goal**,
  in this cycle and after it. vstack detects drift and informs the agent
  at session start; a local refresh brings the repo current. The blocker
  is factual, not a matter of effort: the managed assets are not tracked
  in consuming repos — the lock file is gitignored in all six checked,
  no agents are tracked anywhere, and skills only in four of six — so
  there is nothing to open a pull request about. The invariant that
  follows (never mutate a working tree vstack does not own) is in
  ARCHITECTURE.md, which outlives this file. Evidence:
  [vstack#1254](https://github.com/vanillagreencom/vstack/pull/1254#issuecomment-5274025650).
