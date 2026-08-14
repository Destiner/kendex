# vstack2 v0.2 — the best-of-breed cycle

> **Cycle progress** (updated every commit; deleted sections = landed):
>
> - Phase 0 ✅ (221f950) — changelog, app-deploy skill, commit-msg rule.
> - Phase 1 ✅ (through 2002151) — all four live bugs fixed; typed
>   permission intent; bounded tolerant YAML + injection-proof output;
>   one model-alias table; sealed catalog reads; coalesced config edits;
>   scope canonicalization; schema v2 + journaled migration; typed
>   warnings channel; surface groups with per-tool byte caps and
>   fence-safe splitting; commands→skills on Codex with emitted-mapping
>   lock records; per-tool prose vocabulary; output validators in plan
>   preview; hardened process constructor; byte-faithful + uncompared
>   tests. Three adversarial review rounds folded in (14 + 3 + 18
>   findings). ARCHITECTURE.md carries the durable decisions.
> - Phase 2 ✅ (through 886b2c5) — caps v2 (toggle direction, hook
>   enforcement shown end to end, MCP transports), Gemini CLI and GitHub
>   Copilot fully managed in doc-verified formats, B11 plugin harness
>   targeting, effective-state/inert reporting, cross-read
>   duplicate-definition notes, hook-matcher vocabulary translation.
>   Adversarial review #4 folded (10 findings).
> - Phase 3 ✅ — immutable pinned per-commit source store (B12),
>   marketplace-shaped catalog consumption with plugin/leaf namespacing
>   (B7), catalog metadata + bundle group identity for Phase 4.
>   Adversarial review #5 folded (10 findings incl. repointed-source
>   identity).
> - Phase 4 ✅ — typed reason edges (B6), skill dependencies with
>   suppressions and optional selections, dependency-aware remove with
>   real removal preconditions, preview-first refresh (B10), bundles
>   (catalog + marketplace) with member-of uninstall semantics and
>   hold-backs. Adversarial review #6 folded (offline-removal blocker
>   fixed both-sides).
> - Phase 5 ✅ — safety + quality gates (12 rules, deobfuscation, two
>   scoring paths, Critical-blocks gate, fully-bound content-hashed
>   overrides, vstack check + CI workflow + opt-in real-CLI smoke).
>   Adversarial review #7 folded (8 findings, 4 blockers — gate
>   recalibrated; whole real fleet passes with zero false blocks).
> - Phase 6 implemented (d7976c0, d494b19) — 8 rooms to 6 on the
>   recommended IA: triage Home, Review & apply with safety inline,
>   Library+Catalogs merged (bundles lead the add path), Tools+Projects
>   merged, Customize + Settings kept; Settings gained the safety-caution
>   control the engine already backed. Mock-verified every page in
>   Chromium (before/after shots outside repo), vocabulary audit passed,
>   one primary action per view. IA APPROVED by owner (proceed with recommendation).
>   UI visual polish to Vercel grade landed (fb34766): blue accent +
>   good/warning/critical/info semantic palette through index.css tokens,
>   both themes, status dots + tinted pills, underline tabs, mono
>   identifiers, browser-verified page by page against the owner's four
>   Vercel references (before/after shots outside the tree).
>   Real-app Tauri walkthrough DONE (fc7e573): drove the real window on a
>   headless X display against a sandboxed home — adoption, preview→apply,
>   v0.1→v0.2 migration, project add + folder scan, remove-to-trash,
>   scope-busy error, all verified on disk. It caught a real bug: the
>   app/CLI apply path planned from a schema-normalized manifest and
>   silently skipped the promised "Upgrade vstack.toml" op — fixed
>   both-sides via engine::plan_apply, plus byte-faithful schema-line
>   rewrite hardening; adversarial review folded in.
> - Phase 7 release: repo is release-READY (ARCHITECTURE + per-adapter
>   docs carry every durable decision; CHANGELOG Unreleased carries all
>   12 breaking-register rows; suite green). The release ACT is
>   owner-gated end to end — version bump + tag + CI draft + updater
>   signing keys + publishing + real-repo migration (RELEASING.md
>   User-supplied gates and the release-state memory) — and the closing
>   commit that deletes this file + docs/research/ IS the release. Left
>   for the owner to cut.
> - Standing: subagent reports may arrive only as idle notifications —
>   recover them from the session's `subagents/*.jsonl` (user CLAUDE.md).
>   Engine/render changes get adversarial review before merge. Commits
>   to `crates/`/`ui/` need a CHANGELOG entry (hook enforces).

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

## Decision table 1 — render & adapter mechanisms (landed in Phase 1)

Every row landed with Phase 1 and its evidence moved to
`docs/ARCHITECTURE.md` Decisions; the one remainder is
`<plugin>/<leaf>` namespacing for marketplace-shaped catalogs, which
lands with Phase 3 (per-harness name legality already enforces loader
rules).

## Decision table 2 — quality gates & scoring (directive 7)

Verdicts against HarnessKit @ `461a7a1` and wshobson's validation stack;
evidence in `docs/research/harnesskit.md`.

| Piece | Verdict | Rationale |
|---|---|---|
| Rules-engine shape (trait + registry) | **adopt** (HarnessKit) | One `AuditRule` trait, one registry; add a `remediation` field to findings, drop the trait-level severity (three of their rules already contradict it). Inputs are *typed per kind* (skill tree with byte/file budgets, hook registration, MCP command+args+env+headers, plugin manifest+scripts) — "content" is defined per kind, not assumed. |
| Safety rules (6 content, 3 MCP/permission, 2 plugin) | **adopt, amended** | Port the 11 rules; replace the fenced-code exemption (a real bypass) with "the file a harness loads is scanned at full weight, fences included — a fenced `sh` block in a SKILL.md *is* the instruction; content that is plainly quoting rather than instructing (a blockquote, a skill's supporting files) scans one severity lower; secret/token matches never downgrade anywhere". Secret findings are redacted: the matched token never appears in messages, logs, or UI. Plugin rules run where plugin sources are actually readable (installed files at scan; resolved content at install) and are explicitly n/a elsewhere — a desired plugin is only a settings edit today. |
| Their 5 `cli-*` rules | **skip** | vstack has no installed-CLI ItemKind. |
| Safety scoring math | **adopt** (HarnessKit) | `100 − Σ deductions` (25/15/8/3), first hit full, repeats −1, floor 0 — every deduction names a rule at a location. The aggregate score *warns*; blocking is per-finding (next row), because threshold math alone lets a single Critical (score 75) sail through. |
| Blocking rule | **new** | Any Critical finding blocks on its own, independent of the aggregate; aggregate warn < 80, block < 60. Overrides bind to the exact decision: (installation, rendered-content hash, ruleset version, finding fingerprints) — stale the moment any of them changes, granted by a flag that carries the content hash it was shown with (`--allow-unsafe name@hash`, never a bare name), recorded in the manifest *in the same transaction as the apply they unblock*, visible in Audit. A one-time review must never become a standing bypass. |
| Deobfuscation | **hybrid** | Their invisible-char strip + the parked vstack ideas: NFKC + homoglyph folding, with normalization-that-changes-content reported as a finding (severity calibrated against real catalogs during the phase — legitimate text also normalizes). |
| Quality scoring | **adopt static layer only** (wshobson) | Weighted dimensions + multiplicative anti-pattern penalty, advisory, never blocking; skip the LLM judge, Monte Carlo, Elo, badges, letter grades — wrong cost model for a desktop install path. |
| Structural output validators | **adopt, relocated** | Their `validate_generated.py` checks become Rust validators living beside each adapter, run *inside plan preview* so errors block apply — strictly stronger than their after-the-fact CI. |
| Drift detection | **keep** | Our hash-vs-observation Audit beats their mtime gardener. |
| Fix strings on every finding | **adopt** | The single highest-value idea in either source. |
| Real-CLI round-trip smoke tests | **adopt, re-aimed** | Test *our* surfaces, not theirs: stage the exact personal/project trees vstack emits and run each CLI's local parse/list against them, for all seven adapters (Copilot included). Opt-in dev target + CI, where at least one job installs the freely installable CLIs — a run where every check skipped is a failure to gate, reported as such, never green. |
| Gate placement | **hybrid, one novel piece** | Authoring checks in `vstack check`/`init`. Safety scoring runs in *two* places: on the desired rendered artifact at plan time (that's what gates a fresh install — an uninstalled item has no observation), and on observed content at scan (that's what the Audit page shows). Blocking happens at apply. |
| Score presentation | **adopt** (HarnessKit) | Safety and quality are two scores, never averaged; Audit rows gain a findings column, no new page. |

## Phases

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

**Done when:** rules ported with per-rule tests (the downgrade and what
it does not reach, obfuscation finding, redaction); scores visible
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
