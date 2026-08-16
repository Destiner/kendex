# Tags, logos, approvals, shared skills

Four independent workstreams, agreed with the owner and not yet started.
Delete a section once it lands. Any order; 1 and 2 are quick, 3 and 4 are real
features. All work happens in `/home/method/dev/vstack2` except workstream 1,
which edits the user's own content elsewhere on the machine.

Ground rules that apply throughout: `tools/guard` must pass before you stop,
every behaviour change ships with a test that fails without it, and
`docs/ARCHITECTURE.md` gets amended in the same change that reshapes the code.

---

## 1. Tag the existing skills and agents

The tag feature already ships (`crates/core/src/tags.rs`, invariant 15). What
is missing is tags on the user's actual content. **These files are outside the
repo — they are the user's own skills and agents.** Edit only the
`tags:` line in each file's frontmatter; change nothing else.

Owner's instruction: broad classifications, nothing detailed, most items 5–6
tags at most. Where a file has no `tags:` key, add one directly under
`description:`.

### Where the real files are

| Location | Count | Note |
|---|---|---|
| `~/dev/hyprtrade/.agents/skills/*/SKILL.md` | 22 | the real files |
| `~/dev/hyprtrade/.claude/agents/*.md` | 16 | the real files |
| `~/dev/hyprtrade/.claude/skills/` | 18 | **symlinks** into `.agents/skills` — do not edit |
| `~/.agents/skills/*/SKILL.md` | 2 | agent-browser, find-docs |
| `~/.codex/skills/simplify/SKILL.md` | 1 | |

Roughly 41 real files. Verify with `ls -la` before writing — anything that is
a symlink is already covered by its target.

### Proposed tags

Vocabulary: `review testing docs research planning refactoring debugging
security performance git release data ui integration automation`

**hyprtrade skills**

| item | tags |
|---|---|
| benchmark | performance, testing, data |
| code-quality | review, refactoring |
| decider | planning, docs |
| deep-research | research |
| dep-radar | release, security |
| dev | planning, automation |
| github | git, integration |
| ht-ds-usage | ui, docs |
| iced-charts | ui, data |
| iced-rs | ui |
| linear | planning, integration |
| orch | automation, planning |
| preflight | review, testing |
| price-handling | data |
| project-management | planning, docs |
| reviewer | review |
| review-gate | review, git |
| second-opinion | review, research |
| size-ratchet | review, refactoring |
| trading-design | ui |
| visual-qa | testing, ui |
| worktree | git |

**hyprtrade agents**

| item | tags |
|---|---|
| generalist | refactoring, docs |
| iced | ui |
| planner | planning |
| researcher | research |
| reviewer-arch | review, planning |
| reviewer-correctness | review, debugging |
| reviewer-doc | review, docs |
| reviewer-error | review, debugging |
| reviewer-perf | review, performance, testing |
| reviewer-quality | review, refactoring |
| reviewer-safety | review, security |
| reviewer-security | review, security |
| reviewer-test | review, testing |
| rust | performance, refactoring |
| scout | research |
| tpm | planning |

**personal**

| item | tags |
|---|---|
| agent-browser | automation, testing, integration |
| find-docs | docs, research |
| simplify | refactoring, review |

Read each file's description before writing — these were derived from
descriptions and names, and a file may say something the name does not.
Adjust where the file disagrees; the table is a starting point, not gospel.

### Done when

Open the app, filter the Library by each tag, and confirm the counts look
right. `Problems` should show no "is not a tag" warnings.

---

## 2. Real vendor logos

The owner asked for actual logos. Today `ui/src/components/tool-icon.tsx`
draws geometric stand-ins — recognisable but not the real marks.

1. Fetch each mark from the vendor's own brand or press page (not from
   another product's repo — the marks belong to the vendors, and a third
   party's licence does not grant what is needed). Seven: Claude/Anthropic,
   OpenAI (Codex), Cursor, OpenCode, Pi, Google (Gemini), GitHub Copilot.
2. Save as `ui/src/assets/tools/<harness>.svg`. Strip fixed `width`/`height`,
   keep `viewBox`, so they scale.
3. Rewrite `tool-icon.tsx` to render them. Marks are multi-colour, so the
   `muted` prop can no longer be a `text-*` class — use `opacity-40 grayscale`
   for a tool that is not installed.
4. Keep the `--tool-*` hues: they still drive the chips in `tool-badge.tsx`
   and the table. Check the logo and the hue do not clash badly side by side.
5. Watch the guard's raw-colour ban — it exempts `.svg`, so assets are fine,
   but do not inline hex into `.tsx`.

Size on screen is already right (20px in rows, 12px in chips).

---

## 3. Approve a held-back item and install it anyway

**Decision made by the owner: the approval is shared with the project.** It
is written into that project's `vstack.toml`, so anyone on the repo inherits
it. It binds to the file's content, so any edit to the file re-arms the block.

The engine already does all of this — nothing in `crates/core` needs
inventing. `PlanOptions.accepted` exists, `gate::run` mints the override
(`crates/core/src/engine/gate.rs:85-93`), and `quality::overrides::state`
already reports `active` / `stale` / `absent`, which the UI already renders
(`ui/src/components/safety-findings-blocked.tsx:39-49`). What is missing is a
way to *set* it.

### Work

1. `crates/app/src/audit.rs` — `apply_plan` currently takes
   `(scope, remove_orphans)`. Add the accepted-item list and pass it through
   to `PlanOptions.accepted`. Regenerate bindings:
   `cargo test -p vstack-app -- --ignored regenerate_bindings`.
2. `ui/src/components/safety-findings-blocked.tsx` — each held-back row gets
   an action. Wording matters: it is not "ignore", it is "I have read these
   findings and accept them". Something like **"Accept and install"**, with
   the consequence stated plainly underneath — that the approval is saved to
   the project and everyone on the repo inherits it.
3. Route it through the existing apply flow so the approval and the install
   happen in the same operation, which is what binds the override to the
   bytes that were reviewed.
4. Show an accepted item's state honestly. The `stale` case already has copy
   ("You accepted this before, but …") — make sure an active override reads
   as accepted-by-you, not as a fresh problem.
5. Somewhere to see and revoke them. A section on the Settings page listing
   what has been accepted, per project, with a way to withdraw one.

### Tests

- Core: an override granted at plan time survives into the manifest, and a
  one-byte edit to the file makes it stale. (Check `crates/core/tests/` —
  some of this may exist already; do not duplicate it.)
- UI: accepting from the dialog sends the item through to `applyPlan`.

### Copy

New strings go in `ui/src/lib/copy.ts`, not inline. House style is at the top
of that file. Never claim a state the app has not checked.

---

## 4. Adopt a shared skill without breaking the sharing

**Decision made by the owner: take it over and keep it shared.**

### What is actually going on

`~/.agents/skills/agent-browser` is one directory. `~/.claude/skills/agent-browser`
and `~/.pi/agent/skills/agent-browser` are both symlinks pointing at it. The
scan now resolves links (`crates/core/src/scan/files.rs`), so the app already
sees this as one shared item rather than two.

`adopt()` refuses: `crates/core/src/engine/adopt.rs:44-51` returns
`ForeignSymlink` for any link whose target exists.

### Work

1. `crates/core/src/engine/adopt.rs` — when the original is a symlink whose
   target exists, adopt **the target's content**: capture from the resolved
   path into the scope's local source, and plan removal of the link. The
   follow-up apply re-renders the managed artifact for every harness that
   declares it, which with `method = "symlink"` restores the sharing with
   vstack's copy as the canonical one.
2. Invariant 6 says foreign symlinks are conflicts, not clobber targets. This
   change makes an exception for a link the user explicitly asks to adopt.
   Amend the invariant to say so, or the code will read as violating it.
3. Confirm before doing it. The dialog should name the real folder, name
   every tool pointing at it, and say what will move — the owner picked the
   plain "take it over" option, but this destination is worth showing since
   anything else on the machine pointing at that folder will break.

### Tests

- A skill whose install is a symlink to a shared folder adopts the target's
  content, and the other tool's link still resolves to a real file afterwards.
- A **broken** link still behaves as it does today (declare only).
- A link pointing somewhere vstack has no business touching still refuses.

### Risk to call out to the owner when done

Anything outside vstack's knowledge that points at the old shared folder
breaks. There is no way to detect those.

---

## Reference: what changed in the previous session

- Tags feature: `crates/core/src/tags.rs`, `crates/core/src/scan/metadata.rs`,
  `ObservedItem.tags`, Library filter and chips.
- Design system: `ui/src/components/section.tsx` owns the four type steps;
  `ui/src/lib/layout.ts` owns the two page widths; `ui/src/lib/copy.ts` owns
  product prose; `labels.ts` owns id-to-name vocabulary.
- Perf: commands are `#[tauri::command(async)]`; audit is ~0.69s, down from 3s.
- `docs/ARCHITECTURE.md` invariants 14 and 15 and the Decisions list are the
  written-down version of all of it. Read it first.
