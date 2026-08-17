# Problem triage — review, dismiss, and look back

UX design and screen sketches:
<https://claude.ai/code/artifact/39b40336-322a-4df3-ae86-7f7b85c20bcd>

This plan was reviewed adversarially by Codex (gpt-5.6-sol, xhigh) against
the real code. Its verdict on the first draft was **do not implement as
written**, with two blockers. The full review is quoted where it changed a
decision. Everything below is the revision.

## Why

Review & apply carries five unrelated jobs under one heading, one badge and
one Apply button. Only two are problems — a claim about content needing a
human judgment. The rest is queued work, context, and an invitation to adopt.

The gap: findings the gate does *not* block have **no verb at all**. A real
machine shows three — a destructive command across 14 hooks, 20 plugins from
untracked sources, a skill that reads a credential and sends it somewhere —
and nothing clears them. They are back tomorrow. That is why the page never
finishes.

## The thing to understand before touching anything

The existing accept model claims to bind a decision to exact content. **It
does not.** `content_hash` hashes the audit's *reduced* representation, not
the bytes:

- skill trees stop after 512 KiB or 200 files (`quality/observe.rs:22`)
- symlinks are skipped (`quality/observe.rs:178`)
- binary assets contribute only `path:bytes:` — their bytes are discarded
  (`quality/mod.rs:153`, `engine/gate.rs:187`)
- documents are `String::from_utf8_lossy`-decoded before hashing, so
  different invalid bytes collapse together (`quality/observe.rs:105`)
- plugin input keeps package.json, manifest filenames and a narrow source
  extension list — not the whole plugin (`quality/observe.rs:249`)

Concrete defeat, from the review: an untracked plugin with no manifest and
only `payload.wasm` yields a `plugin-source-trust` finding. Replace the WASM
with arbitrary different bytes — representation, content hash, fingerprint,
key and ruleset are all unchanged, and the decision stays active.

**This is a live hole in Accept today, not just in the proposed Dismiss.**
Phase 1 fixes it. Nothing else in this plan may land first.

## Where decisions live — settled

**In the manifest, in the scope the item belongs to.** No sidecar, no second
home, and no change to where Accept already writes. Owner-confirmed.

There is no single `vstack.toml` to choose between; a decision follows its
item, and the two scopes already have the privacy properties you want:

| Scope | File | Shared? |
|---|---|---|
| Personal | `~/.config/vstack/vstack.toml` (`Env::global_manifest_file`) | no — private to this machine |
| A project | `<project>/vstack.toml` (`Env::project_manifest_file`) | yes — committed, so it travels to anyone who clones |

Why this is right rather than merely convenient: all three dismissal reasons
are claims about *content*, not about one person's tolerance. "I trust this
source" and "the checker got this wrong" are exactly the kind of judgment a
team should share and review; "not a problem here" is already scoped to
*here*. A project decision landing in a diff someone reviews is the desired
behaviour for a security judgment, not a leak.

Three consequences this plan adopts because of it:

- **No free-text notes on a dismissal.** A reason enum only. Personal prose
  must never land in a committed file by accident.
- **Say where a decision came from.** Recorded decisions labels each one by
  the file of record — "from this project" versus "yours" — so an inherited
  suppression is never silent. Inheriting is fine; inheriting invisibly is
  not.
- **Reaping is mandatory** (Phase 2), or a committed file accretes records
  for items nobody has any more.

## Non-goals

- **No time-based snooze.** A standing exemption with a timer is still a
  standing exemption.
- **No rule-level mute**, and no bulk dismissal across differing content
  (see Phase 6).
- **No keyboard `A`/`D` shortcuts.** Single-key safety-changing actions are
  premature; the review called this out and it is cut.
- **No cross-scope bulk action or "Undo all".** A plan belongs to exactly one
  scope (`apply/op.rs:240`) and locks that scope (`apply/mod.rs:99`); there is
  no atomic two-scope mutation.
- **No telemetry.**

## What already landed (do not redo)

- `ui/src/lib/audit-counts.ts` — the one place drift is counted. Note it
  counts only changes + blocked; Phase 4 must extend it to warnings or
  dismissing one will not move any number.
- `ui/src/components/status-note.tsx` — `StatusNote` / `StatusLine` over the
  four semantic tones. Use these; do not hand-roll another error style.

## Phases

One commit per phase minimum; `./tools/guard` green before each. Every
behaviour change ships a test that fails without it — Phases 4–7 included.

### Phase 1 — a review hash that actually means "this content"

Introduce a **`review_hash`**: the complete owned bytes of an installation, or
the exact config entry, independent of scan budgets, symlink skipping and
lossy decoding. It is a decision-binding hash, separate from whatever the
rules read.

Decisions bind to `review_hash`, which replaces `content_hash` on the override
record — it strictly dominates it, and two staleness sources buy no extra
protection. `content_hash` keeps its own job of saying which inputs produced
which findings.

Where bytes genuinely cannot be read, `review_hash` is `None` and **no
decision reads as live**. Today that case hashes a constant reason string, so
such a record stays active forever whatever the content — a bug this fixes.

A hook's registration needs no separate binding: an observed hook is named
`{event}:{matcher}:{command_stem}` (`scan/hooks.rs:34`), so changing the event
changes the item name and therefore the decision key, and the old decision
stops applying on its own. It becomes an orphan record instead, which is what
Phase 2 reaping is for.

`TrustedSource` provenance binding lands in **Phase 2**, not here: it
qualifies a dismissal reason enum that does not exist yet, and an unused
provenance field would be dead code.

**Tests (must fail without it):** same-size binary replacement; bytes changed
past the 512 KiB cap; the 201st file changed; different invalid UTF-8 that
decodes identically; a plugin's WASM, manifest, and an unlisted-extension
source file each changed.

### Phase 2 — storage, schema, atomic ops, CLI

One snapshot per installation, dispositions beneath it — this stores the
proof once instead of repeating hash and ruleset per finding:

```toml
[safety-reviews."skill:example:claude"]
review-hash = "…"
ruleset = 3
[safety-reviews."skill:example:claude".dismissed."<fingerprint>"]
reason = "wrong-call"
dismissed-at = "…"
```

- **Bump the manifest to schema 5** (`manifest/mod.rs`) — Phase 1 already
  took it to 4 when the override record's hash field changed. Two bumps is
  the honest cost of fixing the hash before building on it, and it is free
  before release. `serde(default)` is not enough either time: older builds
  must refuse with `SchemaTooNew` rather than misread the file. Add the key
  to `TOP_LEVEL` in `manifest/validate.rs:20`.
- **`TrustedSource` binds source identity/provenance** here, where the reason
  enum exists. Without it, rebinding a source while rendered bytes stay equal
  keeps the trust.
- **Reap on removal.** `engine/ops.rs:80` cleans declarations, forks and
  choices but not safety overrides today. Fix that gap as part of this.
- **Atomic per scope:** one core call, one journaled `WriteManifest` per
  scope's batch. Mixed-validity batch writes nothing.
- **Server-side revalidation.** The operation takes an expected review hash
  plus exact targets, re-audits, and refuses unless hash and fingerprints
  still match. Review data is cached up to 60s (`ui/src/stores/audit.ts:106`),
  so the UI's view is always potentially stale.
- **CLI parity is mandatory** (`ARCHITECTURE.md:186`). Today `accepted` only
  lists and revokes. Add exact-token dismiss, list, and revoke.

**Tests:** concurrent manifest write yields `PlanStale`; a stale token writes
nothing; schema 3 loads and the first mutation writes schema 4; removal reaps;
CLI output matches GUI state.

### Phase 3 — occurrence DTO and Recorded decisions

`Finding` stays a pure rule observation (`quality/mod.rs:86`) — it is built
before installation context exists. Decision state goes on a relation beside
it, under `ItemSafety`, where scope, item and hash already live.

The UI must never construct a decision key. The backend issues a typed
occurrence: `observation + opaque decision token + decision state`, the token
binding scope, installation, review hash, ruleset and fingerprint.

Then **Recorded decisions** (not "history" — it is a live registry, not an
event log): accepts and dismissals, active / stale / obsolete, with revoke.
Grow `ui/src/components/accepted-overrides.tsx` rather than starting over.
`crates/app/src/audit.rs:297` silently skips unreadable scopes — a view
promising every decision must surface those as errors instead.

Durable reversal ships **before** the first dismissal UI, deliberately.

### Phase 4 — dismiss one finding, and truthful counts

Single-finding dismissal with a reason. Toast Undo must revoke the exact
record version it created — an old toast must never delete a newer dismissal
at the same key.

Counts and the finished state need one shared "reviewable findings"
derivation driving Review visibility, the sidebar/Home/footer counts, scope
summaries, and the finished state. Today `partitionSafety` still bins a
dismissed row into warnings (`group-findings.ts:33`) and `review.tsx:28`
treats any non-empty `view.safety` as active, so dismissing everything would
still never say "Everything is in sync".

**Blocked findings stay visible regardless of dismissal state.** Write down
the state matrix first: can an accepted finding also be dismissed; what a
threshold change from warn to block does to an existing dismissal; what
revoking an Accept does to dismissals under it.

### Phase 5 — observed versus desired

`AuditView.safety` describes installed bytes; desired rows reach the UI only
when blocked (`crates/app/src/audit.rs:45,107`), so non-blocking findings in
*queued* content are dropped. A warning-only fresh install therefore looks
"Ready to apply" with no findings, and only appears under Needs review after
it is applied.

Either expose planned safety rows distinctly, or explicitly scope dismissal to
installed observations and accept review-after-install. Decide and document.

### Phase 6 — two zones, and adoption moves out

Split into separate commits: dialog/action, count projection, layout,
adoption relocation. Do not grow `audit.rs` (325/400) or `group-findings.ts`
(200/250) — split before adding.

- **Needs your decision** — blocked first, then non-blocking findings.
- **Ready to apply** — the plan, behind the existing Apply button.
- **Not managed yet** moves to Library → Installed, which already has that
  section. Link to it from Review.

### Phase 7 — focused review, then bulk

Focused one-at-a-time review comes **before** bulk, reversing the first
draft. Bulk dismissal is allowed **only where every target shares the same
review hash and finding evidence** — the same bytes exposed through several
harnesses, which the rules legitimately treat alike since they do not read
the harness (`ARCHITECTURE.md:131`).

`groupFindings` collapses by display text and then by rule
(`group-findings.ts:57,86`), so 20 unrelated plugins with 20 different
contents render as one concern. One click there would mute a rule across the
fleet whatever the storage shape. Grouping stays presentation-only; the
occurrence tokens from Phase 3 are the decision targets.

## Watch out

- `crates/core` stays pure domain logic — no Tauri, no UI concerns.
- TS bindings are generated; regenerate, never hand-write.
- File caps: 400 Rust, 250 TS/TSX. `quality/mod.rs` is at 374 and
  `crates/app/src/audit.rs` at 325 — split before adding.
- Structural changes update `docs/ARCHITECTURE.md` in the same commit.
- **`crates/cli/tests/deps_cli.rs` intermittently burns 120s and fails.** Not
  random contention: 120s is `DEFAULT_TIMEOUT` in `process/mod.rs:27`, so a
  git subprocess is hanging until the hardened runner kills it. Seen on two
  different tests in that file on different runs
  (`refresh_regenerates_freely_but_asks_before_changing_what_is_installed`,
  `sweeping_takes_the_leftovers_and_a_held_back_dependency_says_so`); each
  passes in ~9s run in isolation. Pre-existing and unrelated to this work —
  re-run the file alone before believing a failure, and do not "fix" it by
  chasing your own change.
