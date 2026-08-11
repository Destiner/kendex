# Content quality gates — HarnessKit's audit engine vs wshobson's validation stack

Sources studied (read-only clones):

| Source | Pin | What was read |
|---|---|---|
| `RealZST/HarnessKit` | `461a7a1` | `crates/hk-core/src/auditor/**`, `models.rs`, `service.rs`, `config.rs`, `marketplace.rs`, `adapter/mod.rs`, `src/pages/audit*.{ts,tsx}`, `crates/hk-cli/src/main.rs` |
| `wshobson/agents` | `c4b82b0` | `Makefile`, `tools/validate_generated.py`, `tools/doc_gardener.py`, `docs/plugin-eval.md`, `plugins/plugin-eval/src/plugin_eval/layers/harness_portability.py`, `tools/tests/test_cli_smoke.py`, `docs/round-trip-results.md` |

File refs below are repo-relative to each clone.

---

## 1. Rules inventory (HarnessKit — all 18)

Registered in `crates/hk-core/src/auditor/rules.rs:25`. Deductions: Critical 25, High 15, Medium 8, Low 3 (`models.rs:240`).

### Content rules — `auditor/rules/content.rs`

| Rule id | Applies to | What it catches | Severity | Fix hint? |
|---|---|---|---|---|
| `prompt-injection` | skill, plugin | 7 regexes: "ignore previous instructions", "disregard prior", "you are now a", "new system prompt", "override system/safety prompt", literal `[SYSTEM]`, raw zero-width chars | Critical | No — message is the regex source |
| `rce` | skill, hook, plugin | `curl\|wget … \| sh`, `base64 -d \|`, `eval(`, `curl > /tmp/… && sh\|chmod` | Critical | No |
| `credential-theft` | skill, hook, plugin | Two-part: reads `.ssh/.env/.aws/credentials/.netrc/.pgpass` **and** has an outbound verb (`curl/wget/http/post/nc`). Both → Critical; read-only → downgraded to High inside `check()` | Critical / High | No |
| `plaintext-secrets` | skill, hook, mcp, plugin | Token prefixes `sk-`, `sk-ant-`, `ghp_`, `gho_`, `AKIA…`, `xoxb-`, `xoxp-`, scanned in MCP env **and** headers, plus every whitespace token in content | Critical | No |
| `safety-bypass` | skill, hook | `--no-verify`, `--yes`, `--force`, `allowedTools: "*"`, "bypass … safety/approval", "disable/skip … confirm/prompt". Backtick-quoted flags are treated as documentation and skipped | Critical | No |
| `dangerous-commands` | skill, hook, plugin | `rm -rf /`, `chmod 777`, leading `sudo`, `mkfs`, `dd of=/dev/`, fork bomb. Hook → High, everything else → Medium | High (Medium off-hook) | No |

All six run per-line through `descriptive_line_mask` (`rules/shared.rs:1`), which **skips every line inside a ``` fence or starting with `>`**.

### Permission / provenance rules — `auditor/rules/permissions.rs`

| Rule id | Applies to | What it catches | Severity | Fix hint? |
|---|---|---|---|---|
| `broad-permissions` | mcp | `--host` with `*` or `0.0.0.0`; filesystem MCP server with a non-`/tmp` root path | High | No |
| `supply-chain` | mcp | `npx` running an **unscoped** npm package (no `@`) — typosquatting | Medium | No (names the package) |
| `unknown-source` | all but cli | Source origin is Local with no URL — not installed by a tool, not in git | Low | No |
| `permission-combo-risk` | all | Network+Env (exfiltration) or Shell+Network (RCE) in the inferred permission set; locates the first matching line for each | High | No |

### MCP / plugin rules

| Rule id | Applies to | What it catches | Severity | Fix hint? | File |
|---|---|---|---|---|---|
| `mcp-command-injection` | mcp | `$(…)` or backtick subshell inside an MCP arg. Deliberately does **not** flag `;` or `\|` (SQL/grep false positives) | High | No | `rules/mcp.rs:15` |
| `plugin-source-trust` | plugin | No `plugin.json`/`package.json`/`.cursor-plugin`/`.codex-plugin` manifest (Low); no tracked git origin (Medium) | Medium / Low | No | `rules/plugin.rs:8` |
| `plugin-lifecycle-scripts` | plugin | `preinstall/postinstall/install/prepare` in package.json; Medium if the script body contains `curl/wget/sh/bash/eval/nc`, else Low | Medium / Low | No | `rules/plugin.rs:70` |

### CLI rules — `auditor/rules/cli.rs` (no vstack analogue; vstack has no "installed CLI" ItemKind)

| Rule id | What it catches | Severity |
|---|---|---|
| `cli-credential-storage` | Credential file mode > `0600` on Unix, or API domains with no known credentials path | High |
| `cli-network-access` | More than 3 distinct API domains | Medium |
| `cli-binary-source` | Installed via `curl`/`wget`/`curl\|sh` → High; unknown method → Medium; npm/pip/brew/cargo → clean | High / Medium |
| `cli-permission-scope` | Child skills span > 3 distinct permission types | Medium |
| `cli-aggregate-risk` | Child skills collectively hold network + filesystem + shell; downgraded to Low when the source is tracked | High / Low |

**No rule in the engine emits a fix hint.** `AuditFinding` is `{rule_id, severity, message, location}` (`models.rs:224`) — there is no remediation field, and neither the CLI nor the UI adds one.

---

## 2. Engine architecture

```
Extension rows (store)
   │  service::audit_extensions  (per-kind content assembly)
   ▼
AuditInput { content, source, file_path, mcp_{command,args,env},
             permissions, cli_meta, child_permissions, pack, … }   ← 18 flat fields
   │  Auditor::audit  →  deobfuscate(content)  (strip invisible Unicode)
   ▼
for rule in rules (all 18, always)  →  rule.check(&input) -> Vec<AuditFinding>
   ▼
compute_trust_score(findings)  →  AuditResult { findings, trust_score, audited_at }
   ▼
store.insert_audit_result  →  Audit page / `hk audit`
```

- **Extension point** is one trait, `AuditRule { id(); severity(); check(&AuditInput) -> Vec<Finding> }` (`auditor/mod.rs:30`). Adding a rule = one struct + one `Box::new` in `rules.rs`. Clean, but the input is a fat struct rather than a per-kind enum, so every rule opens with a `matches!(input.kind, …)` early return.
- **Input assembly is per-kind and lossy** (`service.rs:497`): a hook's "content" is the third colon-field of its name; a plugin's is up to 512 KB of concatenated `.js/.ts/.py/.sh` (JSON deliberately excluded); an MCP's content is empty and only command/args/env are scanned; a CLI's content is empty entirely.
- **Deobfuscation** (`auditor/mod.rs:38`) strips `U+200B–200F`, `202A–202E`, `2060–2064`, `2066–2069`, `FEFF`, `00AD`, `180E`, `FE00–FE0F`, `E0100–E01EF` before rules run. Invisible characters only — **no NFKC, no homoglyph/confusable folding**, and the strip is silent (never itself a finding).
- **Scoring** (`auditor/mod.rs:104`): `100 − Σ deductions`, saturating at 0. First finding per `rule_id` costs its full severity; every repeat of the same rule costs exactly 1. Tiers in `models.rs:265`: Safe ≥ 80, Low Risk 60–79, Needs Review < 60.
- **Presentation**: `src/pages/audit.tsx` groups results per extension across agents (worst score wins), lists failed rules **and** the applicable-but-passed rules per kind, and offers a tier filter. `hk audit` (`hk-cli/src/main.rs:666`) prints a Safe/LowRisk/NeedsReview tally then per-extension findings sorted worst-first.

### Five things that are broken or dead — worth knowing before copying

| Finding | Evidence |
|---|---|
| Fenced code and blockquotes are invisible to all six content rules — a payload wrapped in ``` is not scanned, yet the model still reads it | `rules/shared.rs:9`; tests at `content.rs:463–519` assert this as intended behavior |
| `audit.rules_enabled` config (10 toggles) is never read — `Auditor::new()` always loads all 18 rules; the config also names an `outdated` rule that does not exist | `config.rs:25`, no consumer anywhere; `auditor/mod.rs:68` |
| `audit_batch`'s doc comment promises "batch-level duplicate detection"; the body is `inputs.iter().map(audit).collect()` | `auditor/mod.rs:93` |
| `hk audit --kind/--severity` are parsed and discarded (`_kind`, `_severity`), and the command always exits 0 — unusable as a CI gate | `hk-cli/src/main.rs:666` |
| The whole rule catalog and the scoring math are re-implemented in TypeScript for the UI | `src/pages/audit-utils.ts:5` and `:191` mirror `rules.rs` and `compute_trust_score` by hand |
| `prompt-injection`'s zero-width-character regex can never fire — `deobfuscate` removes those characters before the rule runs | `content.rs:19` vs `auditor/mod.rs:76` |

### Adapter/capability patterns vs `crates/core/src/harness/caps.rs`

| HarnessKit pattern | Already in vstack2? |
|---|---|
| Capabilities **derived from adapter declarations** rather than hand-maintained (`AgentCapabilities::from_adapter`, `adapter/mod.rs:630`) | Partly — vstack's `capabilities()` is an explicit match table; only `observe` is described as adapter-derived. HarnessKit's "if the adapter can't resolve a write path, the capability is false" is a stronger anti-drift property |
| One capability source shared by backend gate and UI gate (`src/lib/agent-capabilities.ts`) | Yes — same idea, and vstack's is generated rather than hand-written TS |
| **Sub-kind capability axis**: MCP remote transport flags (`http` vs `sse`, Codex takes Streamable HTTP but not SSE) | **No** — vstack's table is kind-level; an MCP server that only a subset of harnesses can accept by transport has no representation |
| Per-kind × per-op × per-scope matrix (6 ops × 2 scopes) | vstack's is richer; HarnessKit only models project-install + hooks + MCP transport |

Only the transport-level axis is genuinely new. Worth a note in the v0.2 capability work, not in the quality-gates work.

---

## 3. Scoring comparison

| | HarnessKit trust score | wshobson PluginEval | wshobson validate/garden | Parked vstack "security scoring" |
|---|---|---|---|---|
| **Question answered** | Is this dangerous? | Is this *good*? | Is this well-formed and portable? | Is this dangerous? |
| **Scale** | 100 − Σ deductions, floor 0 | Σ(dimension weight × blended layer score) × 100 × penalty | None — counts of error/warning/info | 100 − Σ severity deductions |
| **Inputs** | 18 regex/structural rules | 7 static sub-checks + 4 LLM-judge dimensions + Monte-Carlo reliability | Parsed frontmatter, JSON/TOML schemas, file sizes, link targets, mtimes | Same shape as HarnessKit |
| **Weights** | Severity → fixed deduction (25/15/8/3) | Explicit per-dimension weights (triggering 25%, orchestration 20%, output 15%, scope 12%, disclosure 10%, tokens 6%, robustness 5%, structure 3%, code 2%, ecosystem 2%) plus per-layer blend weights | n/a | Severity-weighted |
| **Repeat handling** | First hit per rule = full deduction, each repeat = −1 | Multiplicative anti-pattern penalty `max(0.5, 1 − 0.05 × count)` — count of *distinct* flags, floor 50% | n/a | "dedup of repeated identical findings" |
| **Obfuscation** | Invisible-char stripping only | None | None | Invisible chars **+ homoglyph/NFKC folding** |
| **Bands** | Safe ≥80 / Low Risk 60–79 / Needs Review <60 | Platinum 90 / Gold 80 / Silver 70 / Bronze 60 + A+…F letter grades | error / warning / info | block-below-threshold |
| **Gate** | **None** — advisory only, CLI exits 0 | `score --threshold N` exits 1 (CI) | exit 1 on error; `--strict` also on warning | block at install |
| **Fix hints** | None | Yes — every portability finding carries `remediation`, appended to the anti-pattern description | Yes — every finding renders `fix: …` / `Fix: …` | not specified |
| **Cost** | Free, deterministic, < 1s | Free at `quick`; ~4 LLM calls at `standard`; ~54–104 at `deep`/`thorough` (3–6 min) | Free, deterministic | Free |

### Recommended hybrid

**Two scores, never averaged into one.**

1. **Safety score** — HarnessKit's model, kept: `100 − Σ deductions`, severity-weighted, first-hit-per-rule full / repeats −1, floor 0. It is the only one of the three that can justify blocking, because every deduction traces to one named rule at one location. Add the two things HarnessKit lacks:
   - a `remediation: String` on every finding (wshobson's pattern — the fix travels with the error), and
   - homoglyph/NFKC folding on top of invisible-char stripping, with **the obfuscation itself reported as a Critical finding** rather than silently normalized away. Content that needs deobfuscating to look clean is the signal.
2. **Quality score** — wshobson's weighted-dimension model, but static layer only: weights per dimension, multiplicative anti-pattern penalty with a floor. Advisory, never blocking. No LLM judge, no Monte Carlo — the cost/latency does not fit a desktop app's install path, and vstack has no eval corpus.

Reject: HarnessKit's fenced-code exemption (replace with "fenced content is scanned, and a hit inside a fence is one severity lower" — documentation examples become Medium, live payloads stay Critical); the parked "dedup identical findings" as literal de-duplication (HarnessKit's −1-per-repeat is better: a skill with 40 `curl | sh` lines should not score the same as one with a single line); PluginEval's letter grades and Elo (no corpus, no audience).

---

## 4. Where gates should run

| Stage | vstack verb | HarnessKit today | wshobson today | Fit for vstack |
|---|---|---|---|---|
| **Authoring** — scaffold + check a catalog item | `vstack init`, `vstack check` | Nothing | `make validate` (740 LOC, 5 harness validators, error/warning/info + `fix:`), `make garden` (5 drift checks, every finding has `Fix:`) | **Strong fit.** vstack already renders per-harness artifacts, so it can validate what it is about to write: frontmatter completeness, name-matches-directory, Codex's 8 KB skill cap, OpenCode permission keys and name shape, Cursor's three allowed `.mdc` keys, Gemini `{{args}}`. These are exactly the checks that turn a silent runtime truncation into a build error. Exit codes: 1 on error, `--strict` also on warning |
| **Install** — before disk is touched | `vstack apply` (plan shown first) | **Nothing** — audit runs *post*-install (`commands/install.rs:124` "Post-install: scan, sync, set meta, audit"); marketplace shows third-party risk badges from `skills.sh` (`marketplace.rs:602`, ath/socket/snyk) but never blocks | `--threshold N` exits 1, but in CI, not at install | **The genuinely novel piece.** vstack's apply already computes a plan and revalidates preconditions before mutating (invariant 7). The safety score belongs in that plan: show the score and the findings in the plan UI, warn below 80, block below 60 with an explicit per-item override that is recorded in the manifest. Nobody in either source has built this |
| **Diff / audit page** | `vstack diff`, Audit page | The whole product surface — grouped per extension, worst-agent score, passed rules shown alongside failed | n/a | Fits directly. vstack's Audit page is already the declared-vs-observed diff; safety findings are a second column on the same rows, not a new page |
| **CI for the default catalog** | GitHub Actions over the catalog repo | `hk audit` always exits 0 → unusable | `make validate STRICT=1`, `make garden STRICT=1`, `make test`, `make smoke-test` (real CLI: `opencode agent list`, `gemini extensions validate`, `codex doctor`; skips gracefully when a CLI is absent) | **Adopt the exit-code discipline and the smoke test.** wshobson's `docs/round-trip-results.md` records three real bugs the CLI round-trip caught that unit tests missed (YAML block-scalar descriptions breaking OpenCode's loader; an empty `tools:` list generating a deny-everything permission block; OpenCode rejecting a custom `$source` key). vstack renders artifacts for five harnesses and has no equivalent check |

Mapping onto scan → declare → diff → apply: **authoring validation is a `check` concern, safety scoring is a `diff` concern, blocking is an `apply` concern.** No new verb is needed; the score is a property of an Installation, computed during scan and surfaced in the diff.

---

## 5. Adoption notes

### Port to Rust

| Item | From | Notes |
|---|---|---|
| `AuditRule` trait + registry | HarnessKit `auditor/mod.rs:30`, `rules.rs:25` | Keep the shape; add `remediation` to the finding and drop `severity()` from the trait (three rules already return a severity that differs from the trait's, so the trait method is misleading) |
| 6 content rules | `rules/content.rs` | Port verbatim minus the fence exemption; regexes are the accumulated value here |
| `mcp-command-injection`, `supply-chain`, `broad-permissions` | `rules/mcp.rs`, `rules/permissions.rs` | Direct fit — vstack manages MCP servers as a first-class ItemKind |
| `plugin-lifecycle-scripts`, `plugin-source-trust` | `rules/plugin.rs` | Fits Claude plugins; source-trust partly duplicates vstack's lock provenance |
| Score + tiers | `auditor/mod.rs:104`, `models.rs:265` | ~40 LOC. Keep the −1-per-repeat rule |
| Deobfuscation | `auditor/mod.rs:38` | Extend with NFKC + confusable folding, and emit a finding when normalization changes the content |
| Structural validators | wshobson `tools/validate_generated.py` | Per-harness checks belong next to each adapter in `core/harness/`, not in one file — vstack already owns rendering per harness, so the validator is "does my own output satisfy the target schema" |
| `remediation` on every finding | wshobson, everywhere | The single highest-value idea in either source |
| Real-CLI smoke test | wshobson `tools/tests/test_cli_smoke.py` | Skips when a CLI is absent; caught three bugs unit tests missed |

### Skip

- All five `cli-*` rules — vstack has no installed-CLI ItemKind.
- `unknown-source` — vstack's manifest/lock already answers provenance precisely; a Low-severity guess is noise next to a real lock entry.
- `permission-combo-risk` as written — it depends on HarnessKit's inferred `Permission` model (`scanner.rs:1903`, ~200 LOC of content inference producing FileSystem/Network/Shell/Database/Env). Either port that inference layer deliberately or drop the rule; do not port the rule alone.
- PluginEval Layers 2 and 3 (LLM judge, Monte Carlo), Elo, corpus, letter grades, badges.
- `rules_enabled`-style config toggles until something actually reads them.
- Duplicating the rule catalog in TypeScript — vstack generates bindings; the catalog ships as generated types.

### Surface estimate

| Piece | Rules | Rust LOC (impl) | Tests |
|---|---|---|---|
| Engine (trait, input, deobfuscate, score, registry) | — | ~180 | ~120 |
| Safety rules (6 content + 3 mcp/permission + 2 plugin) | 11 | ~550 | ~350 |
| Structural/authoring validators (per-harness) | ~10 checks across 5 harnesses | ~450 | ~250 |
| Quality dimensions (static only, weighted) | 6–7 sub-checks | ~250 | ~150 |
| Plan/apply gate + threshold config + override record | — | ~150 | ~120 |
| **Total** | ~21 rules + ~10 checks | **~1,600** | **~1,000** |

For scale: HarnessKit's whole auditor is 1,641 LOC including tests; wshobson's validator plus gardener is 1,199 lines of Python. A vstack implementation lands in the same order of magnitude, and roughly half of it is regexes and per-harness schema facts that can be transcribed rather than designed.
