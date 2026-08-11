# v1 baseline — dependencies, catalog, provenance, changelog

Ground truth: `~/dev/vstack` (v1) at `d44d5145` (the PLAN-era pin was
`169eff98`). All paths below are relative to that repo. Read-only survey;
nothing in v1 was modified.

Each section ends with **Replace-or-keep** — the question the v0.2 planner
has to answer, stated as what vstack2 would be giving up or inheriting.

---

## 1. The skill dependency mechanism

### 1.1 Schema

Declared in YAML frontmatter of `skills/<name>/SKILL.md`, one optional
`dependencies` block with two string lists:

```yaml
dependencies:
  required: [orch, github, decider]
  optional: [linear]
```

| Element | Type | Ref | Notes |
|---|---|---|---|
| `Skill.dependencies` | `Option<SkillDeps>` | `cli/src/skill.rs:17` | absent → body-scrape fallback fires (§1.2) |
| `SkillDeps.required` | `Vec<String>` | `cli/src/skill.rs:33` | defaults to empty |
| `SkillDeps.optional` | `Vec<String>` | `cli/src/skill.rs:35` | defaults to empty |
| `SkillDep {name, optional}` | flattened pair | `cli/src/skill.rs:38-43` | the two lists collapse into one list with a bool |

Names are bare skill names — no version, no kind, no source qualifier, no
harness scoping. **Only skills have dependencies.** `CatalogKind` has five
variants and agents/hooks/pi-extensions/extras carry no equivalent field
(`cli/src/catalog.rs:19-28`).

Parsing: `Skill::from_file` splits frontmatter, deserializes with
`serde_yaml`, then calls `resolve_dependencies`
(`cli/src/skill.rs:45-57`, resolver at `:289-308`).

### 1.2 The body-scrape fallback (the part to look at hardest)

When frontmatter has no `dependencies` key, v1 falls back to **parsing a
markdown table out of the skill body**: `parse_dependencies_from_body`,
`cli/src/skill.rs:313-419` (106 lines of heuristics).

| Heuristic | Behavior |
|---|---|
| Section detection | any heading containing `"Dependencies"` or `"Skill Dependencies"` opens the section; the next heading closes it |
| Reverse-dep guard | lines containing `self-contained`, `depend on it`, or `Dependent Skill` flip a flag that discards the rest |
| Early exit | a line starting `Project-level` or `**Project` breaks the loop |
| Optionality | the literal substring `(optional)` anywhere in the row |
| Name extraction 1 | backtick-quoted tokens in column 1, alphanumeric/`-`/`_` only, no spaces, not `$`-prefixed |
| Name extraction 2 | if no backticks: strip `(optional)`/`(e.g.,`, take a `"Xxx Yyy skill"` suffix pattern, kebab-case the prefix |

This infers a dependency edge from prose. It is silent on failure and
silent on false positives.

### 1.3 Graph construction and expansion

```
skills[] --build_dependency_graph--> {name -> [required names]} --expand_dependencies--> (expanded[], added[])
                 skill.rs:243-261                                        skill.rs:265-287
```

`build_dependency_graph` (`cli/src/skill.rs:243-261`) does two filters
before an edge exists:

1. `.filter(|d| !d.optional)` — **optional deps never enter the graph**.
2. `.filter(|d| skill_names.contains(...))` — a required dep naming a skill
   not in *this source's* catalog is **silently dropped**. No warning, no
   error. Cross-source dependencies are structurally impossible.

`expand_dependencies` (`cli/src/skill.rs:265-287`) is a plain BFS over that
graph with a `seen` set. Returns `(full set, auto-added subset)`. Cycles
terminate (the `seen` set absorbs them) but are never reported.

### 1.4 What expansion does at install time — three call sites, three behaviors

| Call site | Ref | Behavior |
|---|---|---|
| `vstack add --skill a,b` | `cli/src/commands/add.rs:1959` | expands the filter, prints `Auto-added dependencies: …` to stderr, installs the union |
| Agent install pulls skills | `cli/src/commands/add.rs:1608-1648` (`auto_include_agent_skills`) | resolves `[agent-skills]` + `[role-skills]` for each selected agent, then expands those transitively, pushing missing skills into the install set |
| TUI checkbox | `cli/src/tui/install_flow.rs:1175-1200`, `:1200-1230` | selecting a skill auto-selects **and locks** its required deps; deselecting runs `unlock_orphan_deps` to release deps no longer needed by anything selected |

Optional deps reach the user in exactly one place: a hint string
`"requires: x, y | optional: z"` rendered in the TUI list
(`cli/src/tui/state.rs:~585-615`). Nothing else reads them. `vstack add
--skill` on a CLI never mentions optional deps at all.

### 1.5 Where the mechanism is absent

| Gap | Evidence |
|---|---|
| **`vstack refresh` never re-expands** | the only `expand_dependencies` callers are `add.rs` and `install_flow.rs`. A skill that gains a new required dep upstream does not pull it in on refresh — the install set is frozen at install time |
| **`vstack remove` is dependency-blind** | `cli/src/commands/remove.rs` contains no reference to deps. Removing `github` leaves `dev`, `orch`, `reviewer`, `dep-radar`, `project-management` silently broken |
| **The lock records no reason** | `LockEntry` (`cli/src/config.rs:8-24`) has no "auto-added" / "requested-by" field. Nothing on disk distinguishes "the user asked for `decider`" from "`dev` dragged it in". Uninstall therefore *cannot* be dependency-aware even if someone wrote the code |
| **Dangling required deps are invisible** | §1.3 filter 2. No validation command flags them |
| **`user-invocable: true`** | parsed at `cli/src/skill.rs:14`, set to `None` by every constructor, **read by nothing**. A dead field in 7 shipped skills |

### 1.6 Real examples (all seven declaring skills, verbatim)

| Skill | required | optional |
|---|---|---|
| `orch` | `github, worktree, dev, project-management, decider, reviewer` | `linear, review-gate, second-opinion` |
| `dev` | `orch, github, decider` | `linear` |
| `reviewer` | `orch, github, decider` | `linear` |
| `project-management` | `linear, github` | `decider` |
| `dep-radar` | `github` | `worktree` |
| `iced-shadcn` | `iced-rs` | — |
| `deep-research` | — | `decider` |

11 of 18 skills declare nothing. Note `orch → dev → orch`: a **two-node
cycle** ships in the real catalog. It survives only because the BFS `seen`
set swallows it — installing either one installs both, which happens to be
the intent, but nothing validates or reports the cycle.

Also note the shape of the `orch` list: six required + three optional is
not really "this skill needs that skill" — it is a curated set of nine
items expressed through the only mechanism available. **v1's dependency
graph is already being used as a bundle**, badly.

### 1.7 Replace-or-keep

The frontmatter schema itself (`{required, optional}`, bare names) is
minimal and sound, and the TUI select-and-lock interaction is genuinely
good product behavior worth reproducing. What vstack2 should not inherit:
the body-scrape fallback, silent drops of unresolvable deps, optional deps
that exist only as a tooltip, expansion frozen at install time, a
dependency-blind remove, and a lock that cannot say why an item is
present. The last one is the load-bearing gap for v0.2 — bundle uninstall
("does removing a bundle remove its members?") is unanswerable without a
provenance field recording *why* each installation exists, and v1 has no
such field to migrate from. That is a greenfield decision, not a
replacement.

---

## 2. Parked "kits" notes

**There are none in v1.** Grep for `kit` across `docs/`, `AGENTS.md`,
`README.md`, `CHANGELOG.md`, and the full git log turns up nothing on
topic. The v1 repo never had the concept.

The parked note is vstack2's own, in the now-deleted `docs/PLAN.md`
(recoverable at `git show c5b4eae^:docs/PLAN.md`):

> `| Post-parity: kits/packs, registry browsing, marketplace install, security scoring | — | 7 (post-release) |`

That single table row is the entire prior design record. `docs/PLAN-PROMPT.md:58-66`
restates it as the v0.2 charge. **Replace-or-keep:** nothing to inherit —
bundles are unconstrained by prior v1 commitments.

The nearest v1 behavior to a bundle is `auto_include_agent_skills`
(§1.4): an **agent acts as an implicit bundle root**, dragging in its
mapped skills plus their transitive deps. Its member list lives in the
manifest's `[agent-skills]` / `[role-skills]` tables, not in the agent
file. Whether bundles generalize that or replace it is a live design
question — the two overlap.

---

## 3. Catalog layout and per-kind conventions

### 3.1 Kinds and defaults

`CatalogKind::default_paths` (`cli/src/catalog.rs:19-28`), all repo-root
relative:

| Kind | Default dir | Item shape | Discovery fn |
|---|---|---|---|
| Agents | `agents/` | one flat `<name>.md`, YAML frontmatter | `catalog.rs:226` via `discover_files(…, "md")` `:171` |
| Skills | `skills/` | directory containing `SKILL.md` (+ free-form extra files) | `catalog.rs:244` via `discover_manifest_dirs(…, "SKILL.md")` `:199` |
| Hooks | `hooks/` | one flat `<name>.sh` | `catalog.rs:263` via `discover_files(…, "sh")` |
| PiExtensions | `pi-extensions/` | directory containing `package.json` | `catalog.rs:281` |
| Extras | `extras/` | theme packs / assets | `catalog.rs:299` |

**There is no `commands` kind in v1.** vstack2's `command` ItemKind is
entirely new; there is no v1 convention to inherit or import.

Actual root contents at the pin: 17 agents, 18 skills, 4 hooks (+ a
`hooks/tests/` dir), 15 pi-extensions, `extras/{assets,vanillagreen-themes}`,
and `skill-templates/` (authoring scaffolds — not a catalog kind).

### 3.2 Discovery rules

`discover_manifest_dirs` (`catalog.rs:199-224`) accepts the configured path
**either** as the item dir itself (it contains the manifest file) **or** as
a container whose immediate children are item dirs. Depth is one level —
no recursion. Duplicate names across paths: first wins, later ones
`eprintln!` a skip warning (e.g. `catalog.rs:232-237`).

### 3.3 Layout override

Opt-in `[catalog]` table in the source repo's `vstack.toml`
(documented at `vstack.toml:19-24`, parsed via `MappingConfig`
`cli/src/mapping.rs:7`, presence check `catalog.rs:30-41`):

```toml
[catalog]
agents = ["agents"]
skills = ["skills", "packages/skills/*", "one-offs/specific-skill"]
hooks = ["hooks"]
pi_extensions = ["pi-extensions", "pkgs/plugins/pi-*"]
extras = ["extras"]
```

Constraints, all hard errors (`expand_catalog_entry`, `catalog.rs:63-116`):
relative paths only, no `..`, no absolute, `*` on the **final segment
only**, empty path rejected. Omitted keys keep their defaults.

### 3.4 Replace-or-keep

This is already a multi-root, glob-capable catalog — a wshobson-style
`plugins/<name>/{agents,commands,skills}` repo is *partially* reachable
today by pointing each kind at `plugins/*/agents` etc. What it cannot do:
recurse deeper than one level, treat a plugin directory as a unit, or read
a committed marketplace registry. The 5-kind enum and the "commands don't
exist" hole are the real breaks — both already require new code in
vstack2 regardless of the layout decision.

---

## 4. Manifest and lock: what provenance v1 records

### 4.1 Three files, three jobs

| File | Role |
|---|---|
| `vstack.toml` | user intent — mappings and customization. In the source repo it carries `is_source_catalog = true` (`vstack.toml:11`), which routes per-project install state to a sibling file instead of mutating the catalog |
| `vstack-local.toml` | the source repo's own install state, so a maintainer's local testing doesn't leak downstream. Tables observed: `[agent-launch-instructions]`, `[agent-additional-instructions]`, `[skill-instructions]`, `[agent-skills]`, `[agent-frontmatter.{claude,opencode,codex,pi}]` |
| `.vstack-lock.json` | provenance of what is installed |

### 4.2 Lock schema

`LockFile { version: u32, entries: BTreeMap<String, LockEntry> }`
(`cli/src/config.rs:103-106`). **Keyed by bare item name** — a flat
namespace across all kinds, so `agent:reviewer` and `skill:reviewer` would
collide. `LockEntry` (`cli/src/config.rs:8-24`):

| Field | Type | Purpose |
|---|---|---|
| `name` | String | |
| `kind` | `ItemKind` (skill/agent/hook/pi-extension/extra, `config.rs:28`) | |
| `source` | String | absolute local path of the source checkout at install time |
| `source_repo` | `Option<String>` | GitHub `owner/repo`. Comment at `config.rs:12-14`: durable across moved/absent local paths; used for ownership routing where installed assets have no frontmatter |
| `harnesses` | `Vec<String>` | which harnesses received it |
| `method` | `InstallMethod` (`Copy` \| `Symlink`, `config.rs:87`) | |
| `installed_at` | String | RFC3339 UTC |
| `source_hash` | String | 16-hex content hash. Comment at `config.rs:20-21`: staleness detection instead of mtime, immune to git checkout/rebase |

Live sample (`.vstack-lock.json`, 37 entries — 17 agent, 16 skill, 4 hook):

```json
"dev": { "name": "dev", "kind": "skill", "source": "/home/method/dev/vstack",
         "source_repo": "vanillagreencom/vstack",
         "harnesses": ["claude-code","opencode","codex","pi","cursor"],
         "method": "copy", "installed_at": "2026-08-10T22:53:53Z",
         "source_hash": "d7e465f5cd0a1f42" }
```

Every field is uniform across all 37 entries — no entry omits any.

### 4.3 What the lock does NOT record

No dependency edges. No "auto-added" flag. No requesting item. No group or
bundle. No per-file hashes (one hash per item). No target paths (they are
recomputed from the harness adapter). No version. No enabled/disabled
state. No user-edit detection on the installed copy.

Separately, `SourceRegistry` (`cli/src/config.rs:109-125`) tracks
multi-source state: `current`, `entries`, `removed_entries` (a tombstone
list so a shipped default source is not resurrected after removal), and a
per-project-root last-selected source.

### 4.4 Replace-or-keep

vstack2's Installation model (item × harness × scope) is already finer
than v1's one-row-per-name-with-a-harness-list, and Architecture
invariant 4 already commits to what v1's `source_repo` + `source_hash`
were reaching for. The genuinely good v1 idea to carry forward is
`removed_entries` — a tombstone that stops a default from resurrecting.
The gap v0.2 must fill is a **reason/owner field on each installation**
(user-requested · required-by-`<item>` · member-of-bundle-`<name>`);
without it, both bundle uninstall and dependency-aware remove are
undecidable, and nothing in v1 can be migrated to supply it.

---

## 5. Changelog style

v1's root `CHANGELOG.md` exists (89 lines) but is **not** Keep-a-Changelog
and **not** versioned — the only two headings in the file are
`# Changelog` and `## Unreleased`. v1 never cut a release.

Entry shape: a bulleted list of **bold-titled** paragraphs, each carrying
its tracker refs inline, with impact called out in bold mid-paragraph:

```
- **orch: claude handoff lanes launch autonomous and verify brief delivery**
  (VST-191 / #1173). <2-8 lines of prose: what changed, the setting/key
  involved, and why the old behavior was wrong.>
  **Breaking**: `all` is now a reserved item name — …
  **Migration note for existing installs**: …
```

Written one entry per PR (`git log -- CHANGELOG.md` shows each entry
landing with its feature commit), never batched at release.

The disciplined variant lives per pi-extension:
`pi-extensions/<name>/CHANGELOG.md` (e.g. `pi-extensions/pi-qol/CHANGELOG.md`)
is `# Changelog` → `## Consumer-impacting changes` → `### <semver>` with
terse bullets, bumped in the same commit as the `package.json` version.
`AGENTS.md:233` makes it a rule and `pi-extensions/package-policy.test.mjs`
enforces file presence and shape.

**Replace-or-keep:** vstack2 is required to produce Keep-a-Changelog with a
backfilled `0.1.0` (`docs/PLAN-PROMPT.md:76-80`), so the root-file style is
replaced outright. Two v1 habits are worth keeping: the per-PR write
cadence (not batched at release), and inline **Breaking** / **Migration
note** call-outs inside the entry that names them — v0.2's permitted
breaking changes need exactly that. The enforcement pattern (a policy test
that fails when a consumer-impacting change lands without its entry) is
the strongest idea in v1's changelog practice and has no vstack2
equivalent; `tools/guard` is the natural home.
