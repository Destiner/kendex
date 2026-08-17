# Adversarial review of the problem-triage plan

Codex (gpt-5.6-sol, reasoning effort xhigh), run via the
`second-opinion` skill in challenge mode against the working tree.
Kept verbatim: the plan in `problem-triage.md` is the revision that
answers it, and the citations here are the evidence.

Verdict: do not implement this plan as written. The central “exact content” guarantee is false in the current code, and bulk dismissal would become a rule mute in practice.

## Ranked findings

### 1. BLOCKER — the dismissal can survive materially different content

`content_hash` hashes the audit’s reduced representation, not the complete bytes:

- Skill trees stop after 512 KiB or 200 files ([observe.rs:22](/home/method/dev/vstack2/crates/core/src/quality/observe.rs:22)).
- Symlinks are skipped ([observe.rs:178](/home/method/dev/vstack2/crates/core/src/quality/observe.rs:178)).
- Binary assets become only path + byte count; their bytes are discarded ([quality/mod.rs:153](/home/method/dev/vstack2/crates/core/src/quality/mod.rs:153), [gate.rs:187](/home/method/dev/vstack2/crates/core/src/engine/gate.rs:187)).
- Documents are decoded lossily before hashing, so different invalid raw bytes can hash identically ([observe.rs:105](/home/method/dev/vstack2/crates/core/src/quality/observe.rs:105)).
- Plugin input retains package.json, manifest filenames, and a narrow source-extension list—not the complete plugin ([observe.rs:249](/home/method/dev/vstack2/crates/core/src/quality/observe.rs:249), [observe.rs:263](/home/method/dev/vstack2/crates/core/src/quality/observe.rs:263)).

Concrete defeat: an untracked plugin with no manifest and only `payload.wasm` produces the same `plugin-source-trust` finding. Replace the WASM with arbitrary bytes; the plugin audit representation, content hash, fingerprint, key, and ruleset all remain unchanged. The dismissal remains active.

That directly disproves the safety claim at [problem-triage.md:58](/home/method/dev/vstack2/docs/roadmaps/problem-triage.md:58).

Before dismissals, introduce a full raw `review_hash`: complete owned bytes or exact config entry, independent of scan budgets and lossy decoding. “Trusted source” must additionally bind source identity/provenance.

### 2. BLOCKER — “Dismiss all N” is a rule-level mute at decision time

The UI collapses findings first by display text, then into one concern per rule ([group-findings.ts:57](/home/method/dev/vstack2/ui/src/lib/group-findings.ts:57), [group-findings.ts:86](/home/method/dev/vstack2/ui/src/lib/group-findings.ts:86)). Thus 20 unrelated plugins with 20 different contents appear as one `plugin-source-trust` concern.

Writing 20 content-bound records protects against future changes. It does not prove the user inspected 20 different contents now. One click has still muted a rule across the current fleet.

Allow bulk dismissal only where every target shares the same review hash and finding evidence—for example, identical bytes exposed through several harnesses. Current rules explicitly do not read the harness ([ARCHITECTURE.md:131](/home/method/dev/vstack2/docs/ARCHITECTURE.md:131)), so those duplicates can legitimately share a review.

For different hashes, focused review must precede bulk. Phase 5 is ordered backwards.

### 3. HIGH — the UI cannot safely address what it displays

`FindingGroup` keeps one copied finding and only `{kind,name,harness}` for affected installations ([group-findings.ts:47](/home/method/dev/vstack2/ui/src/lib/group-findings.ts:47)). Its grouping key omits severity, remediation, fingerprint, and content hash.

That is already lossy: `dangerous-commands` assigns different severity by kind ([shell.rs:113](/home/method/dev/vstack2/crates/core/src/quality/rules/shell.rs:113)). Two occurrences may render as one group while requiring different fingerprints. Adding `fingerprint` to the copied `Finding` does not preserve the fingerprint belonging to each affected installation.

The UI needs backend-issued occurrence targets, not key construction:

`FindingOccurrence = observation + opaque decision token + decision state`

The token should bind scope, installation, full review hash, ruleset, and fingerprint. The backend must revalidate it before writing.

### 4. HIGH — Accept and Dismiss do not compose coherently

Accept is described as item-level, but its record covers the entire finding set ([overrides.rs:24](/home/method/dev/vstack2/crates/core/src/quality/overrides.rs:24)); minting records every current finding ([gate.rs:86](/home/method/dev/vstack2/crates/core/src/engine/gate.rs:86)). An active acceptance therefore already means all exact findings were reviewed.

Unanswered cases:

- Can an accepted finding also be dismissed?
- If Accept is revoked, do its dismissals suddenly hide details on the newly blocked item?
- If thresholds change a warning into a block, does the old dismissal still hide it?
- Does Dismiss apply to a medium finding inside an aggregate-blocked item?

Merely promising that dismissal “does not unblock” is insufficient. Blocked findings must remain visible regardless of dismissal state.

Also, filtering dismissed findings does not produce a finished state: the row retains its raw `warn` verdict, `partitionSafety` still places it in warnings ([group-findings.ts:33](/home/method/dev/vstack2/ui/src/lib/group-findings.ts:33)), and Review considers any `view.safety` row active ([review.tsx:28](/home/method/dev/vstack2/ui/src/pages/review.tsx:28)).

Keep `Finding` as a pure rule observation ([quality/mod.rs:86](/home/method/dev/vstack2/crates/core/src/quality/mod.rs:86)). Use one backend review model with explicit effects; the UI can still say “Install anyway” and “Mark reviewed.” One vague UI verb would not fix the state-machine problem.

### 5. HIGH — the plan ignores observed-versus-desired content

`AuditView.safety` describes installed bytes, while desired rows are sent to the UI only when blocked ([audit.rs:45](/home/method/dev/vstack2/crates/app/src/audit.rs:45), [audit.rs:107](/home/method/dev/vstack2/crates/app/src/audit.rs:107)). Nonblocking findings in queued content are discarded.

Consequences:

- A warning-only fresh install appears “Ready to apply” without its findings.
- The user applies it, then it appears under Needs review afterward.
- Dismissing installed content while an update is queued can become stale immediately after Apply.
- A Decisions page cannot assign one state when a record is active against observed bytes but stale against desired bytes.

Either expose all planned safety rows distinctly, or explicitly limit dismissal to installed observations and accept the review-after-install UX.

### 6. HIGH — removal, reinstall, source changes, and forks resurrect decisions

The proposed key contains name/harness/fingerprint, while the review hash excludes provenance ([gate.rs:180](/home/method/dev/vstack2/crates/core/src/engine/gate.rs:180)).

Therefore:

- Remove then reinstall the same name/content: the old dismissal becomes active again.
- Rename away then back: the old record resurrects.
- Change source while rendered bytes remain equal: `TrustedSource` survives without binding the trusted source.
- A fork deliberately keeps the same name while changing provenance to local ([fork.rs:1](/home/method/dev/vstack2/crates/core/src/engine/fork.rs:1)).

Removal currently cleans declarations, forks, and related choices but not safety overrides ([ops.rs:80](/home/method/dev/vstack2/crates/core/src/engine/ops.rs:80)). Adding another unreaped map compounds existing garbage.

Removal, rename, fork, and source-rebind plans need explicit decision lifecycle rules. At minimum, reap removed installations and invalidate provenance-based reasons.

### 7. HIGH — the proposed manifest shape is both noisy and internally redundant

Each record repeats the fingerprint in its map key and body, and repeats the same item content hash/ruleset across every finding. Those two fingerprint copies can disagree after a hand edit.

Worse, per-finding storage buys little isolation: any unrelated item edit or any global `RULESET_VERSION` bump stales every finding record for that item. You pay per-finding manifest growth while retaining item-wide invalidation.

A better shape under the current architecture is one snapshot per installation:

```toml
[safety-reviews."skill:example:claude"]
content-hash = "…"
ruleset = 3

[safety-reviews."skill:example:claude".dismissed."fingerprint"]
reason = "wrong-call"
dismissed-at = "…"
```

That stores the proof once and dispositions beneath it.

The plan also conflates a live registry with history. Undo deletes records, while stale and removed records remain forever. That is not history; it is inconsistent retention. Choose:

- Active registry: reap removed items, replace prior snapshots, label missing items obsolete.
- Audit history: append events and tombstones in a separate generated ledger.

Do not put an append-only history into the hand-authored manifest.

Finally, this is a schema change. `serde(default)` is insufficient. The manifest is schema 3 ([manifest/mod.rs:13](/home/method/dev/vstack2/crates/core/src/manifest/mod.rs:13)), and top-level keys are explicitly validated ([validate.rs:20](/home/method/dev/vstack2/crates/core/src/manifest/validate.rs:20)). Bump to schema 4 so older binaries report `SchemaTooNew`, rather than treating a same-version file as invalid.

### 8. HIGH — concurrency and CLI parity are not optional follow-ups

A plan belongs to exactly one scope ([op.rs:240](/home/method/dev/vstack2/crates/core/src/apply/op.rs:240)); execution locks exactly that scope ([apply/mod.rs:99](/home/method/dev/vstack2/crates/core/src/apply/mod.rs:99)). There is no atomic two-scope mutation.

Therefore:

- Keep every bulk action scope-local.
- Do not offer cross-scope “Undo all.”
- Use one journaled `WriteManifest` for a scope’s batch.
- Revalidate the decision token server-side; Review data may be cached for 60 seconds ([audit.ts:106](/home/method/dev/vstack2/ui/src/stores/audit.ts:106)).
- Toast Undo must revoke the exact record version it created. An old toast must not delete a newer dismissal at the same key.

CLI parity is mandatory: architecture says every core operation has a CLI verb ([ARCHITECTURE.md:186](/home/method/dev/vstack2/docs/ARCHITECTURE.md:186)). Today `accepted` only lists/revokes acceptances ([accepted_cmd.rs:10](/home/method/dev/vstack2/crates/cli/src/commands/accepted_cmd.rs:10)). Dismiss requires an exact-token CLI creation path, listing, and revocation—not only GUI commands.

## Required sequencing

Cut the standalone fingerprint phase. Move durable reversal ahead of the first dismissal UI. Split the page redesign from the behavior.

```text
full review hash + lifecycle/state matrix
                  ↓
core storage + schema 4 + atomic ops + CLI
                  ↓
typed occurrence DTO + Decisions/revoke
                  ↓
single-finding dismissal + truthful counts
                  ↓
two-zone layout / move adoption
                  ↓
focused review; bulk only for identical evidence
```

Also:

- Split the current Phase 3 into at least dialog/action, count projection, layout, and adoption relocation commits.
- Do not “grow” `audit.rs` or `group-findings.ts`: they are already 325/400 and 200/250 lines, while the guard enforces those caps ([tools/guard:25](/home/method/dev/vstack2/tools/guard:25)).
- Cut the `A`/`D` keyboard shortcuts initially. Safety-changing single-key actions are premature.
- Rename “Decisions history” to “Recorded decisions” unless it becomes a real event log.

## Regression tests the plan needs

| Layer | Tests that must fail without the behavior |
|---|---|
| Review identity | Same-size binary replacement; changed bytes past 512 KiB; 201st file changed; invalid UTF-8 bytes changed; plugin manifest/WASM/unlisted source changed |
| Lifecycle | Remove/reinstall; rename away/back; source rebind; fork with ignored-byte changes; `TrustedSource` bound to the actual source |
| State model | Warn→block threshold change remains visible; Accept revoke does not reveal hidden block details; all warnings dismissed produces a genuinely finished Review state |
| Transactions | Mixed-validity bulk writes nothing; concurrent manifest write yields `PlanStale`; stale Undo cannot delete a newer record; cross-scope operations cannot partially masquerade as success |
| Manifest | Schema 3 loads empty; first mutation writes schema 4; unknown/mismatched fingerprint records fail validation; removal reaps records |
| UI grouping | Same display text with different severity/fingerprint preserves every exact target; mixed active/stale/absent occurrences render and count correctly |
| CLI | Exact-token dismiss, reason required, list, revoke, stale token writes nothing, output matches GUI state |
| Decisions | Active/stale/obsolete sorting; unreadable scopes surface an error instead of being silently skipped—the current list skips them ([audit.rs:297](/home/method/dev/vstack2/crates/app/src/audit.rs:297)) |

This test scope is required by the repo rule that every behavior change carries a failing-before test ([CLAUDE.md:10](/home/method/dev/vstack2/.claude/CLAUDE.md:10)).

