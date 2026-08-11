# Prompt: plan the v0.2 "best-of-breed" cycle

You are opening a fresh session in `~/dev/vstack2` to WRITE THE PLAN for
the next major cycle. Your deliverable is `docs/PLAN.md` (plus supporting
research reports) — do not implement anything beyond scaffolding the plan
documents. Read `AGENTS.md` and `docs/ARCHITECTURE.md` before anything
else; they carry the repo rules and the system model.

## Where the product stands

vstack2 is complete through its original six phases: a desktop app
(Tauri + React) and a drop-in-compatible `vstack` CLI over one Rust
engine, managing agents, skills, hooks, commands, MCP servers, plugins,
and Pi extensions across Claude Code, Codex, OpenCode, Cursor, and Pi.
Safety model: declared intent in per-scope `vstack.toml`, lock files,
drift detection, preview-first transactional applies with journal,
rollback, trash, and launch recovery. ~185 Rust tests + 29 UI tests, all
green. `v0.1.0` exists as an unpublished draft release on the private
repo `vanillagreencom/vstack2` (see `docs/RELEASING.md` for the tag-driven
release flow). Real v1-managed repos remain owned by v1 until the user
migrates them; `vstack import` handles migration and was proven against a
copy of a real repo.

Key code geography: `crates/core` is pure domain (no Tauri) — adapters in
`core/src/harness/` with a static capability table (`caps.rs`), per-tool
agent renderers in `core/src/render/agent/`, v1 semantic mappings
(model tiers, tool-name aliases, role skills) in `core/src/mapping.rs`,
the plan/diff engine in `core/src/engine/`, transactional apply in
`core/src/apply/`. The UI (`ui/`) renders state only over generated
bindings; product vocabulary lives in `ui/src/lib/labels.ts` (no jargon:
tools not harnesses, Personal not global, out of sync not drift). UI
validation runs the real pages in Chromium via the `VITE_MOCK=1` bridge
(`ui/src/dev/`) driven by agent-browser.

## The directives for this cycle (from the owner, binding)

1. **Adopt wshobson/agents architecture wherever it is a clear
   improvement.** Treat it as the guide; keep current vstack mechanisms
   only where they are clearly superior. vstack already rewrites content
   per tool (frontmatter shape, model tiers, codex-only fields like
   `sandbox_mode`, tool-name aliases, deny-lists) — the question per
   mechanism is whose version is better, not whether to have one. The
   plan must contain an explicit adopt / keep / hybrid decision table
   covering at least: tool-vocabulary remapping, model-alias mapping
   across vendors, body-size caps with overflow into `references/`,
   commands-converted-to-skills for tools without a command surface,
   permission/allowlist synthesis from a `tools:` declaration, and their
   capability-matrix-drives-every-adapter pattern.
2. **Deep content rewriting is mandatory** for the product to work well.
   Design the render pipeline so one authored file produces genuinely
   idiomatic output per tool — not lowest-common-denominator copies.
3. **Add Gemini CLI and GitHub Copilot as supported tools** (seven
   total). Follow the Phase-1 pattern from the previous cycle: first an
   observation matrix (read-only scan surfaces, detection roots, project
   vs personal locations, config formats) grounded in current official
   docs plus wshobson's adapters (`tools/adapters/gemini.py`,
   `copilot.py`, `docs/harnesses.md`), then capability-gated management.
4. **Unpark kits, named "bundles", and keep dependencies too.** A bundle
   is a curated, installable set of items (agents + skills + commands +
   hooks); a dependency is one item requiring another (v1's
   `dependencies: {required, optional}` in skill frontmatter already
   auto-expands — keep or improve it). Both concepts coexist. Design:
   catalog authoring shape, manifest declaration, lock/provenance,
   UI browsing/install, uninstall semantics (does removing a bundle
   remove its members? decide and justify).
5. **Reconsider the catalog layout.** Compare the current v1-shaped
   catalog (`agents/ skills/ hooks/ …` at the root) against wshobson's
   `plugins/<name>/{agents,commands,skills}` and its committed
   marketplace registries. Consuming wshobson-style marketplace repos
   directly as catalogs would be a major win — evaluate feasibility.
   Breaking changes to v1 catalog/manifest compatibility are PERMITTED
   this cycle, but every breaking change must be listed in
   `CHANGELOG.md`, and `vstack import` plus a documented migration path
   must keep real v1 repos able to move forward.
6. **Create and keep `CHANGELOG.md`** (Keep-a-Changelog style: an
   Unreleased section, one entry per release, breaking changes called
   out). Backfill a `0.1.0` entry. The "update the changelog with every
   feature release" rule must live in a new, INCREDIBLY LIGHTWEIGHT repo
   skill at `.claude/skills/app-deploy/SKILL.md` — a few lines covering:
   bump version, update changelog, tag per `docs/RELEASING.md`. It will
   later grow distro release pipelines (Arch, Fedora, Ubuntu…); do not
   build those now, just leave the seam.
7. **Content quality gates and a scoring framework.** Study both
   wshobson's stack (`make validate` with fix strings, `doc_gardener.py`
   drift detection, the plugin-eval scoring dimensions, round-trip
   real-CLI smoke tests) and HarnessKit's audit rules engine, and pick
   the best approach or hybrid. Decide where gates run: authoring time
   (`vstack init`/`validate` in catalogs), install time (score/warn), CI
   for the default catalog. Consider folding in the previously parked
   security-scoring ideas (severity deductions, repeat dedup, Unicode
   deobfuscation, block threshold) if they fit naturally.
8. **A UI/UX repolish pass comes LAST, after everything above lands.**
   The owner finds the current app experience disappointing: categories,
   layout, and information architecture are not intuitive and the polish
   bar is not met. This pass may restructure navigation and page
   taxonomy (bundles will likely reshape Library/Catalogs), must follow
   the existing plain-language vocabulary rules, must keep exactly one
   primary action per view and confirm-with-preview before any file
   change, and must be validated by driving the real UI in a browser
   (VITE_MOCK) with screenshots, iterating until it genuinely feels like
   a polished consumer product. Use lower-cost agent models for UI
   implementation and validation.

## References (pin these exactly; clone fresh, they are not vendored)

| Repo | Pin | Study |
|---|---|---|
| `wshobson/agents` | `c4b82b0` | `ARCHITECTURE.md`, `tools/adapters/*` (esp. `base.py`, `capabilities.py`, `codex.py`, `gemini.py`, `copilot.py`), `tools/{generate,validate_generated,doc_gardener}.py`, `docs/{harnesses,architecture,plugin-eval,authoring}.md`, plugin/marketplace manifests |
| `~/dev/vstack` (v1) | `169eff98` | READ-ONLY. Parity baseline already implemented; consult only to settle "is current vstack clearly superior" questions |
| `RealZST/HarnessKit` | `461a7a1` | the audit rules engine (for directive 7); adapter/capability patterns already adopted |
| `runkids/skillshare` | `14959e2` | only if adoption/provenance questions resurface |

Write one consumable research report per studied repo into
`docs/research/` (deleted when the cycle ends), including a
surfaces/behavior matrix for Gemini CLI and Copilot verified against
current official documentation — wshobson's adapters are a strong lead
but not ground truth for tool behavior.

## What still binds (unchanged from the last cycle)

- Engine invariants: preview-first, transactional/journaled applies, user
  edits never clobbered, removals durable, unmanaged files never touched,
  provenance never silently rebound, one writer per scope, capability
  table gates every action.
- `crates/core` stays pure domain; UI renders state and invokes commands;
  TS bindings are generated, never hand-written.
- Every behavior change ships with a test; `tools/guard` (pre-commit) is
  the enforcement list; file line caps apply (split, don't compress).
- Plain-language UI vocabulary; no eng-speak in any user-visible copy.
- The relaxation: v1 CLI/manifest compatibility is no longer sacred —
  breaking is allowed with CHANGELOG entries and a working migration.

## What the plan you write must contain

- Phases, each ending with a working app and a green suite, with concrete
  "done when" acceptance per phase; research first, UI pass last, then a
  `v0.2.0` release (changelog, tag, draft) per `docs/RELEASING.md`.
- The adopt/keep/hybrid decision tables for directives 1 and 7, with a
  one-line rationale per row. Where a decision genuinely needs the owner
  (product trade-offs, naming, catalog-compat scope), list it in an
  "Open questions" section with product-framed options — do not bury
  decisions inside implementation prose.
- A breaking-changes register feeding CHANGELOG.md.
- Explicit boundaries for parallel agent work (disjoint file ownership),
  adversarial review for engine/render changes, and cheap-model guidance
  for UI work, mirroring what worked last cycle.
- The plan document itself is consumable: items get deleted as they land,
  and the file dies at the end of the cycle.

Practical notes: session-specific harness quirks (subagent report
recovery, WebKit-on-Wayland launch env) are documented in the user-level
CLAUDE.md and `crates/app/src/lib.rs`. The app runs from
`target/release/vstack-app`; the UI validation loop is
`VITE_MOCK=1 npx vite --port 5273` + agent-browser. Delete this prompt
file in the same commit that lands the finished `docs/PLAN.md`.
