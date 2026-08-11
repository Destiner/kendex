# wshobson/agents — mechanism study for v0.2

Subject: `wshobson/agents` @ `c4b82b0` (2026-08). 91 plugin dirs, 204 agents,
180 skills, 109 commands, 5 target harnesses. All paths below are relative to
that repo unless prefixed `crates/` (vstack2).

Read this as: what they built, what we have, and whether to take it.

---

## 1. Generation pipeline end-to-end

One authored file becomes per-tool output in four hops. Nothing in the source
tree carries harness conditionals — `ARCHITECTURE.md:11` makes that an invariant.

```
plugins/<plugin>/agents/<name>.md          authored: Claude-Code markdown,
plugins/<plugin>/skills/<n>/SKILL.md       YAML frontmatter + body
plugins/<plugin>/commands/<n>.md
        │
        ▼  base.load_plugin()                          tools/adapters/base.py:335
   PluginSource { agents[], skills[], commands[] }     parse_frontmatter → base.py:41
        │        AgentSource.tools/.model/.color are computed properties (base.py:241)
        ▼
   HarnessAdapter subclass                             base.py:409 (ABC)
     .capabilities → CAPABILITIES[harness_id]          capabilities.py:51
     .emit_plugin(plugin)  → per-plugin artifacts
     .emit_global(plugins) → marketplaces, context-file checks
     returns EmitResult { written[], skipped[], warnings[] }
        │
        ▼  generate.py --harness X --all [--clean --prune --strict]
   .codex/{skills,agents}/ · .opencode/{agents,commands,skills}/
   .copilot/{agents,skills,commands}/ · skills|agents|commands/ (gemini)
   + committed registries: .agents/plugins/marketplace.json, .cursor-plugin/,
     plugins/*/.codex-plugin/plugin.json, gemini-extension.json
```

Transforms all live in the adapter layer and all read the same capability row:
frontmatter rewriting, model-alias mapping, body-size caps, tool-name remapping.
Transformed trees are **gitignored**; only tiny registries pointing back at
`plugins/` are committed (`docs/harnesses.md:70-94`).

The vstack equivalent, for orientation:

```
source catalog (git|path)  →  source.rs:131 source_config / :191 find_item
        ▼
engine/desired.rs:131 desired_state  →  desired_agent.rs (parse → merge → render)
        ▼  render/agent/mod.rs:210 generate  →  per-harness fn
   Artifact::{File,Tree,Registration}         desired.rs:33
        ▼  engine/item_plan.rs (diff vs observation, hash-based)
   apply/ (journaled, transactional, locked)
```

The shapes rhyme; the difference is what happens *after* rendering. wshobson
writes to disk and prunes; vstack plans, hashes, diffs against observation, and
applies transactionally under a scope lock. Their pipeline has no lock, no
adopt, no user-edit drift concept. Ours has no lint on what it emits.

---

## 2. Mechanism comparison

| Mechanism | wshobson | vstack2 today | Lean |
|---|---|---|---|
| **Capability matrix drives adapters** | `capabilities.py:13-172` — frozen `Capability` dataclass per harness carrying *format facts*: `skill_body_max_bytes`, `tool_name_case`, `bare_model_aliases`, `context_file_max_lines`, plus feature flags. One table read by adapters (degradation), the docs generator (`docs/harnesses.md`), and plugin-eval's `harness_portability` dimension. Hand-maintained; nothing proves it matches the adapters. | `crates/core/src/harness/caps.rs:79` — `capabilities(harness, kind) -> KindCaps`, purely *operational* (observe/adopt/install/toggle/remove/refresh × project/global). Test-enforced: `harness/mod.rs:147` asserts the observe column equals what adapters actually declare as surfaces, and `:174` asserts no mutation exceeds observation. Zero format facts — those are literals scattered in each renderer. | **hybrid** — keep the op table and its tests; add a sibling format table in the same file so caps, not renderers, own byte caps / name case / model dialect. |
| **Tool-vocabulary remapping** | `capabilities.py:176-244` `TOOL_NAME_MAPS`: 5 harnesses × 11 tools, Codex mapping to action verbs ("open the file"), others to lowercase identifiers. Applied two ways: identifier swap (`opencode.py:79`, `copilot.py:53`) and prose rewrite with a conservative regex requiring "the … tool" (`codex.py:185`). `base.py:474` `strip_claude_tool_refs` is the shared fallback. Same regex is mirrored in the lint (`harness_portability.py:_TOOL_PROSE_PATTERN`) so lint and rewriter never disagree. | Only *manifest* tool names are normalized, never body prose: `render/agent/claude.rs:83` `claude_tool_name` (lowercase → PascalCase, so a deny actually matches), `render/agent/opencode.rs:78` `permission_name` (tool → permission key). Bodies pass through byte-for-byte to every harness (`codex.rs:113`, `opencode.rs:125`, `cursor.rs:18`). | **hybrid** — adopt the identifier swap (mechanical, safe, generated file). Do **not** silently rewrite prose; emit a warning instead. Zero value until v0.2 ingests foreign catalogs; today vstack's own sources are written for the fleet. |
| **Model-alias mapping** | `capabilities.py:255-298` `MODEL_ALIASES`: full 5×5 grid (`fable`/`opus`/`sonnet`/`haiku`/`inherit` per harness), with tier separation (Codex: `opus→gpt-5.5`, `sonnet→gpt-5.4-mini`). `resolve_model()` at `:306` returns `(model, warning)`; unknown aliases fall back to `inherit` and the warning lands on `EmitResult.warnings` so the user learns their choice was overridden. Cursor's whole column is `inherit` — they gave up on model selection there. | Two unconnected functions. `render/agent/mod.rs:13` `model_id_for(provider, model)` handles exactly `"openai"` and `"claude-code"`; every tier collapses to one id (`opus|sonnet|haiku → openai/gpt-5.6-sol`). `render/agent/codex.rs:55` `codex_model` is a *second* hardcoded table for the bare form. No warning on unknown alias. **Bug:** `model_id_for("openai", "inherit")` returns `"openai/inherit"` — an invalid OpenCode model id, which is exactly what a foreign catalog will hand us. | **adopt** — one table keyed `(harness, alias)`, tier-preserving, returning a warning on miss. Fixes the `inherit` bug and deletes the duplicate table. |
| **Body-size cap + `references/` overflow** | `codex.py:275` `_split_body_if_oversized`, cap 7400 B (8192 minus headroom). Two-stage: split on `## ` headings that are **outside fenced code blocks** (`:217` tracks the exact backtick run), then a UTF-8-codepoint-safe hard cut preferring a newline (`:249`). Appends a pointer note to the head, writes the tail to `references/details.md`, or `_overflow.md` when the source already has a `details.md`. Warns every time it fires. | Nothing. `grep` for a byte cap across `crates/core/src` returns zero hits. `render/skill.rs:14` `render_skill` copies the whole source tree verbatim and injects `[skill-instructions]`; a 40 KB SKILL.md installs to Codex and gets silently truncated at load. | **adopt** — this is a live correctness bug for the Codex harness, not a nice-to-have. The fence-aware split and the UTF-8-safe cut are both worth porting as written. |
| **Commands → skills where no command surface exists** | Codex deprecated `~/.codex/prompts/`, so `codex.py:523` synthesizes a skill per command, with first-order collision handling (skill and command share a name → `__command` suffix) *and* second-order (`<x>__command` already exists as a real skill → `__cmd`), each warned (`codex.py:349-365`). Copilot does the same with `user-invocable: true` + `disable-model-invocation: true` so it shows in the VS Code `/` menu but never auto-loads (`copilot.py:154`). Gemini transpiles to TOML instead (`gemini.py:124`). | `caps.rs:95,104,118` mark Command `observe_only` for Codex/OpenCode/Cursor. Commands install on Claude only. Honest, and consistent with the ARCHITECTURE rule "never shimmed" — but the user loses commands on 4 of 5 harnesses. | **hybrid** — allow Codex only, where the vendor itself says skills replaced prompts (so it is a native surface, not a shim). Keep OpenCode/Cursor observe-only. Take the collision-suffix logic with it. |
| **Permission / allowlist synthesis from `tools:`** | `opencode.py:87` `_build_permission_block` — the table in its docstring is the whole design: no `tools:` key = unrestricted (emit nothing); `tools: []` = deny-everything-but-`skill`/`task`; `tools: Read, Grep` = allow those + base, deny the other 14; MCP-only list = emit nothing (MCP arrives via server config). `skill`/`task` are *always* allowed because Claude authors never list them. Codex has no allowlist, so `codex.py:494` infers `sandbox_mode` instead: no `tools:` → `workspace-write`; all-read-only subset → `read-only`; else `workspace-write`. Copilot/Gemini remap the list through `TOOL_NAME_MAPS`. | Inverted and deny-only by design. There is no allowlist to synthesize: `parse_source_agent` (`render/agent/mod.rs:83-101`) doesn't read `tools:` at all. Denies are computed from role and name — `claude.rs:65` always denies `Agent` + `AskUserQuestion` (planner exempt), `opencode.rs:58` denies `task`/`question`, `pi.rs:99` denies the whole subagent family plus `tasks_write` for reviewers. Codex sandbox comes from role: `codex.rs:45` gives every Engineer `danger-full-access`. | **hybrid** — vstack's deny-only default is the safer model and stays. But the missing-vs-empty distinction and the read-only inference must be adopted **before** ingesting foreign catalogs; see §3 blocker B4 for why that is a privilege-escalation bug, not a fidelity nit. |
| **Namespacing + name legality** | Every artifact is `<plugin>__<leaf>` (`codex.py:443`, `opencode.py:225`, `gemini.py:87`); `base.py:335` refuses a plugin name containing `__`. OpenCode gets `<plugin>-<leaf>` validated against `^[a-z0-9]+(-[a-z0-9]+)*$` with a 64-char limit, and a cross-plugin id collision **raises** rather than overwriting (`opencode.py:154,200`). A separate CI script, `tools/check_agent_name_collisions.py`, keeps the source tree collision-free. | Flat names throughout; the manifest key *is* the name (`manifest/mod.rs:150`). Cross-source collision is a hard error by invariant 4, but there is no per-harness legality check — an item named `My_Skill` renders to OpenCode and is rejected by its loader with no warning from us. | **adopt** — required the moment a multi-plugin catalog lands (204 agents across 91 plugins will collide). Needs a naming decision at the manifest-key level. |
| **Per-harness frontmatter field stripping** | `codex.py:35-52` — explicit `_CLAUDE_ONLY_SKILL_FIELDS` / `_CLAUDE_ONLY_AGENT_FIELDS` sets dropped before emit, because "Codex silently ignores them; stripping is honest". | Not needed — every renderer builds frontmatter from scratch rather than filtering an input dict. | **keep** — ours is structurally better. |
| **YAML scalar quoting discipline** | `codex.py:136` `_yaml_scalar` quotes on: empty, leading/trailing space, YAML special lead char, `": "`, `" #"`, leading digit, or membership in a 24-entry YAML-1.1 reserved-word set (`yes`/`no`/`on`/`off`/`null` in every case variant). | `render/agent/opencode.rs:143` `yaml_str` quotes only when the first char isn't alphanumeric or the body contains `:#"'\n\t`. A description of `no` or `2 approaches` emits unquoted and type-coerces. Claude's renderer always quotes the description, so it's safe there. | **adopt** — 20 lines, removes a whole class of loader bug. |
| **Warnings channel with fix strings** | `EmitResult.warnings` (`base.py:400`) accumulates per-plugin, printed inline by `generate.py:294`, and `--strict` turns any warning into exit 1. Every validator/lint finding carries a `remediation` / `fix` string — `ARCHITECTURE.md:13` makes it an invariant. | `DesiredState.notes` (`engine/desired.rs:59`) exists but only carries "source unreadable" / "item missing". Renderers cannot report anything. | **adopt** — extend notes into per-item render warnings surfaced in the plan preview; it's the delivery vehicle for every other lint on this list. |
| **Orphan pruning** | `generate.py:131` `prune_orphans` — anything in the output tree not in this run's `written` set is deleted, gated on `--all` (needs a complete view) and on zero errors. Plus `_validate_output_root` (`:66`) refusing to wipe outside the repo or a temp dir. | Lock-driven: `desired.rs:62` `processed` marks declarations whose source resolved, and `engine/removal.rs` sweeps lock entries nothing produced — with the deliberate carve-out at `desired.rs:77` that an *unreadable* source never uninstalls a working artifact. | **keep** — ours is provenance-based, theirs is set-difference; ours survives a broken source file. |
| **Context-file budget management** | Codex adapter validates the committed `AGENTS.md` against a 32 KiB hard cap and a 150-line convention (`codex.py:388-425`); the gardener re-checks `AGENTS.md`/`GEMINI.md`/`CLAUDE.md`. Progressive disclosure is invariant #5. | Context files are not an `ItemKind` (`model.rs:66`) and vstack never touches them. | **keep** — out of scope. Worth revisiting only if "manage CLAUDE.md" becomes a feature. |
| **Large-body file injection** | `gemini.py:188` — command bodies over 4 KB are not inlined; the TOML prompt emits `@{plugins/<p>/commands/<c>.md}`, Gemini's native file-injection syntax, resolved at evaluation time against the extension root. | vstack's analogue is `skills_prose` (`render/agent/mod.rs:230`), which lists skill paths for the agent to read. Same idea, applied to skills rather than command bodies. | **keep** — already have the pattern where it matters. |

---

## 3. Catalog & marketplace layout, and can vstack eat it directly?

### Layout

```
.claude-plugin/marketplace.json          # registry, 95 entries
plugins/<name>/
├── .claude-plugin/plugin.json           # 91 present
├── .codex-plugin/plugin.json            # generated, committed
├── agents/<agent>.md
├── commands/<command>.md
└── skills/<skill>/
    ├── SKILL.md
    ├── references/                      # 146 of 180 skills have one
    └── assets/
```

**`marketplace.json` exact shape** (field counts over all 95 entries):

| Level | Field | Presence | Notes |
|---|---|---|---|
| root | `name` | required | `"claude-code-workflows"` |
| root | `owner` | required | `{name, email, url}` — Cursor 2.5 rejects without it |
| root | `metadata` | required | `{description, version}` |
| root | `plugins[]` | required | |
| entry | `name` | 95/95 | |
| entry | `source` | 95/95 | `"./plugins/<n>"` (91) **or** `{source:"git-subdir", url, path}` (4) |
| entry | `description` | 95/95 | |
| entry | `version` | 95/95 | must equal `plugin.json.version` — CI-enforced (`test_cli_smoke.py:182`) |
| entry | `author` | 95/95 | `{name, email?, url?}` |
| entry | `license` | 95/95 | |
| entry | `category` | 95/95 | free string; 26 distinct in use |
| entry | `homepage` | 91/95 | |
| entry | `keywords` | 6/95 | |

`plugin.json` is the npm-ish subset: `name`, `version`, `description`, `author`,
`license` (all ~91/91), rarely `homepage`/`repository`/`keywords`/`category`.

### Feasibility: could vstack consume such a repo as a catalog?

**Maps cleanly**

| wshobson | vstack | How |
|---|---|---|
| repo itself | `Source` | `[sources.wshobson] repo = "wshobson/agents"` works today — `remote.rs:9` `clone_url` and `:45` `sync` already clone `owner/repo` into the source cache. |
| `agents/*.md`, `skills/<n>/SKILL.md`, `commands/*.md` | `Item{Agent,Skill,Command}` | Exactly the layout `source.rs:191` `find_item` already expects — just one directory level deeper. |
| plugin dir list | `SourceConfig.agent_dirs` / `skill_dirs` | `source.rs:143` already reads `[catalog] agents=[...] skills=[...]` from a source-side `vstack.toml`. |
| provenance | `LockEntry.source_repo` + `source_hash` | `lock.rs:27` — no change. |
| `references/`, `assets/` | `Artifact::Tree` | `render/skill.rs:14` already copies the whole subtree. |

**Doesn't map**

| Gap | Detail |
|---|---|
| **B1 — no glob in catalog dirs** | `source.rs:203` does a plain `root.join(dir)`. `plugins/*/agents` needs glob expansion, and `list_items` (`:220`) needs to carry the plugin segment back into the item name. Small, mechanical. |
| **B2 — the plugin as a unit disappears** | vstack installs *items*; wshobson ships *plugins*. `ItemKind::Plugin` is observe+toggle only (`caps.rs:86`) and `PluginDecl` (`manifest/mod.rs:109`) holds nothing but `enabled` — provenance lives in the harness's own registry, which `scan/plugins.rs:53` reads. Flattening 91 plugins into 493 items loses group install/remove/enable and the user's mental model. This is a product decision, not a code gap. |
| **B3 — nowhere to put catalog metadata** | `category`, `version`, `author`, `homepage`, `license`, `keywords`, and the plugin description have no home. `ObservedItem` (`model.rs:129`) carries only `description`; `ItemDecl` (`manifest/mod.rs:54`) carries source/harnesses/method/enabled. A browse-and-install surface needs a new catalog-item struct that is *not* the manifest. |
| **B4 — agent frontmatter mismatch, and it is a safety bug** | `parse_source_agent` (`render/agent/mod.rs:83-101`) reads `name`/`description`/`model`/`role`/`color`/`effort` and **silently ignores every other key**, `tools:` included. A wshobson agent with `tools: Read, Grep` therefore imports with no restriction at all — and with no `role:` it defaults to `Role::Engineer` (`mod.rs:36`), which `codex.rs:45` renders as `sandbox_mode = "danger-full-access"`. A read-only reviewer becomes a full-access agent. Must be fixed before any foreign catalog is enabled. |
| **B5 — `model: inherit` breaks OpenCode** | `model_id_for("openai", "inherit")` → `"openai/inherit"`. wshobson uses `inherit` as its portability-recommended value, so this hits immediately. Fixed by the model-table adoption above. |
| **B6 — 4/95 entries are `git-subdir`** | `remote.rs:45` clones whole repos; there is no sparse/subdir checkout. Either skip those entries or clone-and-subpath. |
| **B7 — no version pinning** | `SourceDecl` (`manifest/mod.rs:31`) is `repo`/`path`/`enabled`, and `remote.rs:51` hard-resets to `origin/HEAD`. Marketplace entries carry a `version` we cannot honor, and a refresh can silently change every installed item's content. Needed for reproducibility regardless of this integration. |
| **B8 — name collisions** | 204 agents over 91 plugins, with a dedicated CI script upstream to police it. Manifest keys are flat strings; adopting a namespacing convention (`plugin/name` or `plugin__name`) is a schema-visible decision. |

**Verdict:** reading a wshobson-shaped repo as a *source catalog* is roughly B1
+ B4 + B5 + B8 of work — days, not weeks, and B4/B5 are bugs worth fixing
anyway. Treating `marketplace.json` as a *registry* (browse, categories,
versions, install-as-a-group) is a different and much larger feature: B2 + B3 +
B7, plus UI.

---

## 4. Quality gates stack

Three make targets, all wired into CI (`.github/workflows/validate.yml`), plus a
scoring framework and a real-CLI job.

### `make validate` → `tools/validate_generated.py` (740 lines)

Yes, it emits fix strings. `Finding{severity, harness, path, message, remediation}`
renders as `[error] codex: <path>: <message>\n    fix: <remediation>` (`:33-44`).
Sorted by severity → harness → path. Errors exit 1; `--strict` also fails on warnings.

| Harness | Checks |
|---|---|
| codex (`:114`) | agent TOML parses via `tomllib`; required `{name, description, developer_instructions}`; `sandbox_mode ∈ {read-only, workspace-write, danger-full-access}`; `SKILL.md` frontmatter present, `name` == directory, description non-empty; **file ≤ 8192 B as an error** (promoted from warning because Codex truncates silently and the fix is mechanical); `AGENTS.md` ≤ 150 lines (warning) |
| cursor (`:228`) | `marketplace.json` parses and has `owner`; every entry has `source` (not `path`/`url`); per-plugin manifests parse and have `name`; `.mdc` frontmatter keys ⊆ `{description, globs, alwaysApply}` — the remediation literally says "`agentRequested:`, `mode:`, `tags:` are folklore" |
| opencode (`:382`) | `opencode.json` has `$schema` (info); agent `mode ∈ {primary, subagent, all}`; `model` contains `/` (warning); `permission:` block re-parsed from **raw** frontmatter (the tolerant parser flattens nested maps) — unknown keys and non-`allow/ask/deny` values are errors; skill `name` == dir, matches the safe-name regex, ≤ 64 chars, description non-empty |
| gemini (`:526`) | command TOML parses; has both `description` and `prompt`; `{{args}}` present (warning); skill `name` == dir; model looks like `gemini-*` (warning); `GEMINI.md` ≤ 150 lines |
| copilot (`:608`) | agent/skill/command frontmatter present with non-empty `name`/`description`; non-index commands have a non-empty body |

### `make garden` → `tools/doc_gardener.py` (459 lines)

Recurring drift detection; every finding carries a `Fix:` line; prints a
per-`(severity, kind)` count summary before the findings.

| Check | What it does |
|---|---|
| `stale` (`:81`) | Reverse-maps every generated artifact to its source and compares mtime (1 s grace). The reverse map is the interesting part — it has to undo `<plugin>__<leaf>`, and for Codex it disambiguates real skill vs command-as-skill vs the `__command`/`__cmd` collision suffixes (`:113-132`). Also detects OpenCode skill-id collisions while building the map. |
| `context` (`:260`) | `AGENTS.md`/`GEMINI.md` ≤ 150 lines, `CLAUDE.md` ≤ 200 |
| `links` (`:276`) | Relative markdown links from `docs/` and the four top-level guides that don't resolve — **error** severity |
| `codex-cap` (`:313`) | *Source* skills over 8 KB with no `references/` dir |
| `marketplace` (`:333`) | Entries with a local `source` but no `plugins/<n>/` (error); plugin dirs absent from the registry (info). Deliberately distinguishes local from `git-subdir` so externals aren't flagged. |

### `plugin-eval` — quality scoring (`plugins/plugin-eval/`, `docs/plugin-eval.md`)

Three layers: **static** (<2 s, free, deterministic), **judge** (~30 s, 4 LLM
calls, anchored rubrics), **monte carlo** (50–100 runs, Wilson / bootstrap /
Clopper-Pearson intervals). Depths `quick`/`standard`/`deep`/`thorough` select
which layers run and set the confidence label.

Ten dimensions, each blended across the layers that ran and renormalized:

| Dimension | Weight | static / judge / MC blend |
|---|---|---|
| `triggering_accuracy` | 25% | 0.15 / 0.25 / 0.60 |
| `orchestration_fitness` | 20% | 0.10 / 0.70 / 0.20 |
| `output_quality` | 15% | 0.00 / 0.40 / 0.60 |
| `scope_calibration` | 12% | 0.30 / 0.55 / 0.15 |
| `progressive_disclosure` | 10% | 0.80 / 0.20 / 0.00 |
| `token_efficiency` | 6% | 0.40 / 0.10 / 0.50 |
| `robustness` | 5% | 0.00 / 0.20 / 0.80 |
| `structural_completeness` | 3% | 0.90 / 0.10 / 0.00 |
| `code_template_quality` | 2% | 0.30 / 0.70 / 0.00 |
| `ecosystem_coherence` | 2% | 0.85 / 0.15 / 0.00 |

`Final = Σ(weight × blended) × 100 × anti_pattern_penalty`, penalty
`= max(0.5, 1 − 0.05 × count)`. Badges Bronze/Silver/Gold/Platinum at 60/70/80/90
gated on Elo too; A+…F letter grades.

Static's seven sub-checks are separately weighted: `frontmatter_quality` 32%,
`orchestration_wiring` 23%, `progressive_disclosure` 14%,
`structural_completeness` 10%, `token_efficiency` 9%, `ecosystem_coherence` 6%,
`harness_portability` 6%. That last one (`layers/harness_portability.py`) is the
piece relevant to us — it emits `SKILL_OVER_CODEX_CAP` (15%), `CLAUDE_TOOL_REFS`
(2–10%), `CLAUDE_TOOL_PROSE` (5%), `AGENT_NAME_COLLISION` (10%),
`BARE_MODEL_ALIAS` (3%), each with a `remediation` string, and its regexes are
deliberately identical to the adapter's rewriter so lint and transform agree.

### Round-trip / real-CLI smoke (`tools/tests/test_cli_smoke.py`, `docs/round-trip-results.md`)

Each class skips when its CLI is absent; CI installs OpenCode + Gemini so those
become required gates. No API keys — every command is local-only.

| Harness | Command | Assertion |
|---|---|---|
| OpenCode | `opencode agent list` in a staged tmpdir | exit 0, **and** every `<plugin>__<agent>` from source appears in the output |
| Gemini | `gemini extensions validate <repo>` | exit 0 / "successfully validated" |
| Codex | `codex doctor`; `tomllib.loads` on every agent TOML | exit 0; zero parse failures |
| Claude | `claude --version`; parse `marketplace.json` | `owner` and `metadata.version` present |
| — | marketplace ↔ plugin.json version drift | no mismatches |

`docs/round-trip-results.md:18-38` is the payoff and worth reading directly: the
real CLIs caught three bugs pure unit tests missed — YAML block-scalar
descriptions (`description: >`) producing strings that started with a literal
`>` and broke OpenCode's loader; permission blocks degrading to deny-everything
when `tools:` held only MCP entries, making agents inert; and OpenCode rejecting
a custom `$source` key in `opencode.json`. Their own "coverage limits" section
(`:147`) is honest that none of this proves the *model* selects the skill.

### What vstack has by comparison

| Gate | vstack |
|---|---|
| pre-commit | `tools/guard` — the repo's stated enforcement list |
| manifest validation | `crates/core/src/manifest/validate.rs` (353 lines) — validates the **manifest**, with findings, before load. No equivalent for emitted artifacts. |
| capability honesty | `harness/mod.rs:147,174` — caps must match declared surfaces; nothing may be mutable where it can't be observed. wshobson has no equivalent; their matrix is prose. |
| render correctness | Per-renderer unit tests asserting emitted substrings (e.g. `codex.rs:179`, `opencode.rs:189`) — good coverage of intent, none of *loadability*. |
| drift detection | The Audit page **is** drift, hash-based against observation (`engine/mod.rs:348`). Strictly better than the gardener's mtime heuristic. |
| structural validation of output | none |
| real-CLI smoke | none |
| quality scoring | none |

**Leans on the gate stack:** adopt a Rust `render::validate` pass modeled on
`validate_generated.py` and run it *inside plan preview* so errors block apply
rather than being discovered post-write — that is strictly stronger than their
after-the-fact CI check, and the plan preview is the natural place for findings
that carry fix strings. Adopt the cheap real-CLI smoke subset (`opencode agent
list`, `gemini extensions validate`, `codex doctor`, TOML parse) as an
opt-in developer target. **Keep** ours for drift and orphan sweeping — hash and
lock beat mtime and set-difference. **Defer** plugin-eval entirely: it scores
authoring quality, which is a catalog-author concern, not a
customization-manager concern. If it ever surfaces, it's a read-only quality
column on a browse page, not a gate.

---

## 5. Surprises worth knowing

1. **Their most sophisticated mechanism is bypassed in the shipped install path.**
   The Codex marketplace entry points at the *source* plugin dir
   (`codex.py:590`, `"skills": "./skills/"`), so a real Codex install reads the
   unsplit `SKILL.md` and truncates at 8 KB anyway. The cap-splitting under
   `.codex/skills/` only serves the manual `~/.codex/skills` symlink recipe.
   `docs/harnesses.md:100` admits it. Adopt the mechanism, not the plumbing.
2. **The capability matrix is descriptive, not enforced.** Nothing proves
   `CAPABILITIES` matches adapter behavior; a stale row silently produces wrong
   output. vstack's `observe_capabilities_match_declared_surfaces` test is the
   thing they're missing, and it's the reason to extend our table rather than
   import theirs.
3. **The whole pipeline is one-way and stateless.** No lock, no provenance, no
   adopt, no concept of a user having edited generated output. Regenerate-from-
   scratch plus `--prune` is the entire state model. Every hard problem vstack
   solved (invariants 2, 4, 6, 7, 8) simply does not exist there — so there is
   nothing to learn from them about apply, and everything to learn about *what
   to emit*.
4. **Cursor's model column is entirely `inherit`.** After researching it they
   concluded per-agent model selection isn't honored, and encoded the surrender
   in the table rather than pretending. Our `cursor.rs:5` reaches the same
   conclusion by dropping model/skills/hooks outright.
5. **A `fable` tier already exists in their alias grid** (`capabilities.py:256`),
   with authoring guidance at `docs/authoring.md:130` about when it's worth the
   cost. Our `Role`/`model` vocabulary has no equivalent tier.
6. **Their tolerant frontmatter parser is 110 lines and still lost a round-trip
   bug** (block scalars, `docs/round-trip-results.md:23`). vstack's
   `parse_source_agent` is 33 lines and far more permissive — it `continue`s on
   any line without a colon and ignores unknown keys. For our own catalogs that's
   fine; for foreign ones it's blocker B4.
