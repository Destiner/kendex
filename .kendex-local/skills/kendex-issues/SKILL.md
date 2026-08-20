---
name: kendex-issues
description: >
  Steward the vanillagreencom/kendex issue queue (Linear team KEN is the poll
  surface; GitHub stays the PR/code surface) on a self-paced loop: watch open
  PRs, poll, triage (dedupe; close non-kendex issues and repost project-local
  ones to their owning repo; fix genuine defects in kendex's
  skills/agents/hooks/pi-extensions or the Rust CLI), run each fix through the
  orch skill, merge, propagate via kendex refresh, then reschedule. A thin
  wrapper: fix cycles belong to orch, PR mechanics to the github skill, PR
  monitoring to review-gate's pr-watch, Linear ops to the linear skill — this
  skill carries only the kendex-specific stewardship knowledge. Use when asked
  to monitor kendex's issues continuously or to run one fix-and-propagate
  cycle.
---

> **Never edit this file directly.** To make additions or modifications, edit the appropriate section in the managing project's kendex config — `kendex.toml` at the kendex project root, or `kendex-local.toml` in a source-catalog checkout. Then run `kendex refresh`.

# kendex Issue Steward

Keep the vanillagreencom/kendex issue queue clean and its consumers in sync:
watch → poll → triage → fix → merge → propagate → repeat. One cycle per turn,
then reschedule.

**This skill is a thin wrapper.** The mechanics live in four owning skills,
and every cycle uses them instead of hand-rolling:

| Concern | Owning skill | Load when |
|---------|--------------|-----------|
| Fix cycle (prepare → delegate → review → submit → merge) | **orch** | every fix, before creating the worktree |
| PR threads / replies / reviews / merges / CI logs | **github** | any PR mutation or read |
| Watching open PRs across the loop | **review-gate** (`pr-watch.sh`) | first action, every cycle |
| Linear reads and writes | **linear** | any Linear operation |

A session driven by this skill should load those naturally at each step and
produce zero hand-rolled PR mechanics (raw `gh api` GraphQL where a
`github.sh` command exists is the failure smell).

## Loop (one turn)

1. **PR watch first** — `GH_REPO=vanillagreencom/kendex
   .agents/skills/review-gate/scripts/pr-watch.sh`. Silence + exit 0 means no
   open PR needs you. Attention lines (threads-open, changes-requested,
   gate-stale, disarmed, awaiting-stale) → act through the github skill.
   NEVER hand-roll a PR monitor: pr-watch is the monitoring primitive for
   every open-PR concern except queue ejection, which it cannot see — after
   any gate-green, additionally check `isInMergeQueue` + `autoMergeRequest`
   (GraphQL; `gh pr view --json` lacks queue membership). Ejection is SILENT
   and discards the auto-merge arm: re-arm once; a second ejection means a
   flaky required suite — quarantine/escalate rather than re-arm loops.
2. **Poll Linear** — creation sync is ONE-WAY GitHub→Linear (owner reverted
   the 2026-08-09 two-way experiment), so Linear (team KEN) is the only
   complete queue: Linear-native issues never appear in `gh issue list`.
   Refresh the cache first, then read open issues:
   `linear.sh sync --reconcile`, then
   `linear.sh cache issues list --state "Triage,Backlog,Todo,In Progress" --max`.
   GH-synced issues arrive in **Triage** — omitting it hides the entire
   synced queue (bit the 2026-08-03 campaign: 14 pending issues invisible).
   Then cross-check GitHub: `gh issue list --repo vanillagreencom/kendex
   --state open` — cheap, and history earned it (2026-08-09: nine stray
   open GH issues over eight "empty" cycles while both a generate-on-merge
   automation and two-way creation sync were briefly on; both are off now,
   but pre-revert GH copies of Linear issues still exist and can stick
   open). An open GH issue whose Linear mirror is DONE is residue — close
   it with the diagnosis. An open GH issue with NO Linear mirror means the
   GH→Linear sync broke — triage it from GitHub directly and flag the sync.
   Zero attention lines and zero open on both surfaces → reschedule, stop.
3. **Triage** each issue; run the **Fix cycle** for each valid defect.
   Mutate on the issue's home surface: a GH-mirrored issue (its Linear body
   links the GitHub issue) is closed/commented on GitHub and sync updates
   Linear; a Linear-native issue (no GH link) is updated with the linear
   skill (`issues update` / `comments create` / `issues complete`). PRs,
   branches, and CI stay on GitHub either way. The PR body carries BOTH
   references when the issue has a GitHub mirror — `Closes KEN-<n>` for the
   tracker and `Closes #<n>` for the mirror — so GitHub links the PR on the
   issue and closes it with that reference at merge; a Linear-native issue
   gets `Closes KEN-<n>` alone. Never close a mirror by hand without naming
   the PR in the closing comment.
4. After any merge, **Propagate + Sync**.
5. **Schedule the next wakeup** (see Cadence). Stop only if the user asked.

## Triage (per issue)

Read body + comments, then classify:

- **Duplicate** → close, referencing the canonical issue.
- **Not a kendex asset** → verify by grepping the kendex repo; ownership comes
  from the asset's own SKILL.md frontmatter (`source: kendex`), never from its
  install location. Close with a specific rationale. Project-local assets:
  repost to the owning repo, cross-link both ways, and notify that repo's
  overseer (tmux tab 1: `memsira:1`, `drovr:1`, `hyprtrade:1`) that the fix is
  theirs. The steward never fixes a consuming repo's own defect, however easy
  it looks.
- **Speculative / one-project architecture** → close with reasoning; offer a
  scoped proposal only if it generalizes.
- **Genuine kendex defect** (skills/, agents/, hooks/, pi-extensions/, cli/) →
  Fix cycle. A guidance gap is fixable even when the root trigger is a harness
  limitation; note the harness part upstream.
- **Unfindable references** → check history (`git log -S '<name>' --all`)
  before assuming a rename or regression. A name that never existed in any
  version is a recorded typo: close with that evidence; never ship a
  compatibility shim for it.
- **Empty-body issues** → don't guess the defect from the title. Comment
  asking for the concrete repro, note what your sweep found, leave open.

## Fix cycle (per valid defect) — load orch

The whole cycle is orch's domain: prepare → delegate → review → submit →
merge. Load the orch skill and follow its workflows; do not re-implement any
step here. kendex-specific parameters orch can't know:

- **Agent fit for delegation**: `generalist` (shell/docs/skills), `rust`
  (cli/), `iced` (iced-rs). Require tests and the relevant suite green
  (`bash skills/orch/tests/run-all.sh`, per-skill `tests/*.test.sh`, or
  `cd cli && cargo test`).
- **Fix direction rides in every delegation** when the issue touches skills
  or tooling: determinism and tooling first — a deletion, a short-circuit,
  or a tool; added prose is the last resort. Skills are instructions, not
  explanations (AGENTS.md § Rules, "Engineer over patch").
- **Review the diff yourself** before submit — confirm the actual root
  cause, not a plausible guess. If a delegate stalls, inspect its worktree
  and nudge once.
- A clearly-coupled real defect found along the way → file a tracking issue
  and fix it too.
- Disjoint files → run issues in parallel; same file → sequence or bundle.
- A flaked required check reporting "cannot be rerun" gets a fresh head
  (`git commit --amend --no-edit` + `push --force-with-lease` — only on
  never-shared heads; never amend a pushed commit others may have seen),
  never a force-merge past red.

PR reads and mutations inside the cycle (threads, replies, resolution,
merge, CI logs) go through the github skill: `pr-data --actionable`,
`post-reply`, `resolve-thread`, `pr-merge`, `ci-logs`, `await-mergeable`.

## Propagate + Sync

**BATCH trains (owner directive 2026-08-12): before opening consumer
refresh PRs, scan the open queue (Todo/Triage/in-flight PRs) for items
that touch VENDORED assets — if any would force another re-vendor soon,
HOLD propagation until those land, then run ONE train carrying
everything.** Per-merge trains cost a review round in every consumer each
time; the fail-closed direction of vendored fixes makes the wait safe
(consumers sit at most one train behind, failing loud, never open).
Exceptions worth an immediate train anyway: a fix for a fail-OPEN defect
in a consumer-enforced gate, or an owner ask.

**Only after the upstream change is MERGED and on `origin/main`.** A PR that
is approved, queued, or mid-merge-queue has not propagated anything: merges
can take a while, and refreshing a consumer from an unmerged or mid-queue
state vendors bytes main may never contain. Before any consumer refresh,
verify the source being read (the cache, or the recorded remote clone) sits
at the `origin/main` tip that contains the merge commit — never propagate
from a feature branch, a local unpushed main, or a stale cache.

After merges:

1. Sync local main (`checkout main && pull --ff-only`) and fast-forward the
   cache (`git -C ~/.kendex/cache/vanillagreencom_kendex pull --ff-only`) —
   some consumers source from the cache, and a stale one silently no-ops
   their refresh. Confirm the cache tip equals `origin/main` tip before
   refreshing any consumer.
2. Skill/agent/hook changes → `kendex refresh` (default all scopes) + `kendex
   verify` in each consumer that vendors the asset. Never narrow to
   `--scope project`: Pi packages install at global scope, so a
   project-scoped refresh leaves them drifted and only `kendex verify`
   catches it (bit the 2026-08-09 batch: pi-task-panel stayed at the old
   version through a project-scoped refresh in all five consumers).
   CLI-only changes need a binary rebuild instead, not a skill refresh.
3. Committing in a consumer:
   - Probe first: `git check-ignore -q <refreshed path>` → ignored means a
     local-only install mirror with nothing to commit. The probe is
     authoritative over any remembered per-repo list.
   - Stage only kendex paths (scoped `git add`, never `-A`). Revert no-op
     `.kendex-refreshed` churn and template-default churn on tracked
     `kendex.settings.toml`/`kendex.toml` unless a key change is the payload.
   - Merge-queue repos: branch → PR → `gh pr merge --auto` (no strategy
     flag). Force-merge (`--admin --squash`) is pre-authorized only for
     kendex-only refresh PRs with `kendex verify` green and the required CI
     check green or unaffected; it cannot override a failing required check —
     there, arm `--auto` and let the normal flow merge. Reply to and resolve
     any bot threads either way.
   - Confirm each push landed and the commit contains only kendex files.
   - **While propagation PRs are open, sweep each consumer with
     `GH_REPO=vanillagreencom/<repo> pr-watch.sh --heal` every cycle** — the
     writer's event triggers occasionally miss a bot review (observed
     2026-08-11: hyprtrade-io#43 sat gate-pending ~10 min on a delivered
     review until the cron floor), and `--heal` converges a stale gate in
     seconds instead of waiting out the */15 cron (which itself can slip to
     ~25 min). Same sweep surfaces thread and disarm states on YOUR
     propagation PRs; other PRs' findings route to that repo's overseer.
4. **New skills don't propagate via refresh** — refresh reinstalls only locked
   items. A new skill needs a one-time `kendex add --skill <name> -y` per
   consumer (always `-y`; the interactive prompt dies in non-interactive
   shells), and the `kendex.toml` entry it writes committed through the
   repo's queue where that file is tracked.
5. **Review-bot findings on propagation PRs**: a real defect in vendored
   content is fixed upstream first — issue → fix → merge → refresh on the PR
   branch → resolve threads citing the fix. Stale-doc nits on the PR's own
   payload may be fixed directly on the branch.
6. **Capability changes need repo-side wiring.** A change consumers must
   recognize in their own CI or branch protection is only half-propagated by
   refresh: ship the spec in the owning skill's docs, coordinate the repo-side
   change through each overseer, and verify end-to-end on each consumer's
   origin/main. Security-relevant capabilities need explicit user sign-off
   before propagation.

### This skill's own home

Canonical copy: `~/dotfiles/.agents/skills/kendex-issues/SKILL.md`, installed
PROJECT-scoped in `~/dev/kendex` only — never global. Edit the dotfiles copy,
then run `kendex refresh --scope project` in `~/dev/kendex`. If a stray
`~/.agents/skills/kendex-issues` symlink ever appears, delete it — it collides
with the project install.

## Guardrails

- **Never pipe state-changing or guard commands through `tail`/`head`/`grep`**
  inside a `&&` chain — the pipe replaces the exit status and masks refusals.
  Run bare, check the result, then trim. Always read `worktree create`'s full
  output before cd'ing in; an ownership refusal means another session owns the
  work — back off and coordinate.
- **Multi-agent ownership.** Before a fix cycle, check for an existing remote
  branch, open PR, or foreign local worktree for the issue — other
  orchestrators work this queue concurrently. Owned elsewhere → hands off —
  but ownership expires: a kendex PR with unresolved review threads and no
  pushes or replies for ~2 hours whose authoring session shows no activity
  (its checkout back on main, no worktree) is ABANDONED — take it over,
  answer every thread, and shepherd it to merge, noting the takeover in the
  replies. Hands-off is for active peers, not stalled branches (bit
  2026-08-09: #1139 sat 16 h with five unanswered bot threads while every
  poll skipped it as "the other session's work").
- **Consuming-repo boundary.** The steward's only write lane into consumers is
  kendex propagation (vendored refreshes and kendex settings keys, through the
  repo's own review/queue flow) plus hygiene on those PRs. Everything else
  belongs to that repo's overseer — coordinate over tmux and verify delivery
  (capture-pane after ~5s). Before pressing Enter on a stuck composer, read
  its full content — any text that isn't your own just-sent message means
  don't press Enter; surface the collision instead.
- **Branch safety.** Never switch or commit on a checkout that's on a
  non-default branch or mid-work; use a dedicated worktree off origin/main.
- **Scoped commits, verified pushes.** Inspect diffs for mangling, run
  `kendex verify`, confirm changes actually landed.
- **Box load poisons suites.** Sibling sessions load this machine (stress
  harnesses, ollama); a red bun/bash suite under load is suspect — check
  uptime, rerun in isolation, A/B against clean HEAD before blaming the
  change.
- **Clean up** merged worktrees and stale local branches; keep local and
  remote main in sync everywhere.

## Cadence

Self-pace the next wakeup: widen toward ~60 min when the queue has been quiet;
tighten toward ~15–20 min after a new issue or a burst. Stay in the prompt
cache when actively watching; go long when idle.
