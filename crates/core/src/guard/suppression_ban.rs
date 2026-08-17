//! suppression-ban — two gates over lint suppression in tracked files:
//! blanket (file- or module-wide) suppressions fail flat, and reasonless
//! rust `allow(dead_code)/allow(unused*)` attributes ratchet per file
//! against a tighten-only baseline. Scans index content, language-scoped
//! by pathspec, so documentation that quotes these pragmas never fires.

use std::collections::BTreeMap;

use crate::error::Result;

use super::settings::{Policy, config_path};
use super::{GuardCtx, Outcome, baseline, grep_lane, patterns};

const CHECK: &str = "suppression-ban";

/// The whole parenthesized span must be lint names, commas, and spaces — a
/// `reason = "…"` component carries `=` and quotes, which keeps the
/// attribute out of the bare count.
const BARE_ERE: &str = r"#\[allow\([[:space:]]*([a-z_:]+[[:space:]]*,[[:space:]]*)*(dead_code|unused(_[a-z_]+)?)([[:space:]]*,[[:space:]]*[a-z_:]+)*[[:space:]]*\)\]";

fn blanket_lanes(ctx: &GuardCtx, excludes: &patterns::Excludes, out: &mut Outcome) -> Result<()> {
    let rust_remedy = format!(
        "delete the module-wide attribute and fix the findings, or annotate the surviving sites per line with a stated reason; vendored trees belong in {} with a reason",
        excludes.file
    );
    let lanes = [
        super::Lane {
            label: "module-wide rust allow",
            ere: r"^[[:space:]]*#!\[allow\(",
            remedy: &rust_remedy,
            pathspecs: &["*.rs"],
        },
        super::Lane {
            label: "file-level noqa",
            ere: r"^[[:space:]]*#[[:space:]]*(ruff|flake8):[[:space:]]*noqa",
            remedy: "drop the file-level directive; a per-line noqa naming its specific codes stays legal",
            pathspecs: &["*.py"],
        },
        super::Lane {
            label: "blanket eslint-disable",
            ere: r"/\*[[:space:]]*eslint-disable[[:space:]]*\*/",
            remedy: "name the rules being disabled (and why), or fix the findings; the bare block form turns the linter off wholesale",
            pathspecs: &[
                "*.js", "*.jsx", "*.ts", "*.tsx", "*.mjs", "*.cjs", "*.mts", "*.cts", "*.vue",
                "*.svelte",
            ],
        },
        super::Lane {
            label: "blanket nolint",
            ere: r"//[[:space:]]*nolint([[:space:]]*$|:[[:space:]]*all([^a-z]|$))",
            remedy: "name the linter per line (nolint:lintname with the reason alongside), or fix the finding",
            pathspecs: &["*.go"],
        },
    ];
    for lane in &lanes {
        grep_lane(ctx, CHECK, lane, excludes, out)?;
    }
    Ok(())
}

/// Per-file counts of reasonless allows over index content, excluded paths
/// dropped. A path the baseline TSV cannot carry (a tab or newline in the
/// name) is refused after excludes apply, so the refusal's own remedy —
/// exclude it — works.
fn bare_counts(ctx: &GuardCtx, excludes: &patterns::Excludes) -> Result<BTreeMap<String, u64>> {
    let mut counts = ctx.grep_counts(CHECK, BARE_ERE, &["*.rs"])?;
    counts.retain(|path, _| !excludes.is_excluded(path));
    baseline::assert_baseline_representable(CHECK, &counts)?;
    Ok(counts)
}

fn ratchet(
    baseline: &BTreeMap<String, u64>,
    counts: &BTreeMap<String, u64>,
    baseline_file: &str,
    out: &mut Outcome,
) {
    for (path, count) in counts {
        match baseline.get(path) {
            None => out.violation(
                format!(
                    "suppression-ban FAIL new bare allow: {path} — {count} reasonless allow(dead_code)/allow(unused) attribute(s), no baseline row"
                ),
                "state a reason on each attribute or fix the code; freezing a legacy count is a hand-added baseline row in this diff with justification",
            ),
            Some(frozen) if count > frozen => out.violation(
                format!(
                    "suppression-ban FAIL bare allows grew: {path} — {count} attribute(s) > baseline {frozen}"
                ),
                "state a reason on each new attribute or fix the code; raising the row is a hand-edit in this diff with justification",
            ),
            Some(frozen) if count < frozen => out.violation(
                format!(
                    "suppression-ban FAIL baseline looser than reality: {path} — baseline {frozen} > actual {count}; the ratchet only moves down"
                ),
                &format!("run suppression-ban --update to tighten the row in {baseline_file}"),
            ),
            Some(_) => {}
        }
    }
    for (path, frozen) in baseline {
        if !counts.contains_key(path) {
            out.violation(
                format!(
                    "suppression-ban FAIL stale baseline row: {path} — no bare allows remain (or the file left the tracked, non-excluded set); the row ({frozen}) must go"
                ),
                &format!("run suppression-ban --update to drop the row from {baseline_file}"),
            );
        }
    }
}

/// `--update` rewrites the worktree baseline tightening only: rows lower
/// or vanish, never appear or rise. Growth stays a visible hand-edit.
fn tighten(
    ctx: &GuardCtx,
    baseline_file: &str,
    baseline: &BTreeMap<String, u64>,
    counts: &BTreeMap<String, u64>,
    out: &mut Outcome,
) -> Result<BTreeMap<String, u64>> {
    let updated = baseline::tighten(baseline, counts, |_, _| None, out);
    let target = ctx.root.join(baseline_file);
    if baseline.is_empty() && !target.is_file() {
        out.say(format!(
            "suppression-ban --update: no baseline at {baseline_file} and --update never adds rows; nothing written"
        ));
        return Ok(updated);
    }
    crate::fs::atomic_write(&target, &baseline::render_baseline(&updated))?;
    out.say(format!(
        "suppression-ban --update: baseline tightened at {baseline_file} ({} row(s))",
        updated.len()
    ));
    Ok(updated)
}

pub fn run(ctx: &GuardCtx, policy: &Policy, update: bool) -> Result<Outcome> {
    let baseline_file = config_path(
        CHECK,
        &policy.string(CHECK, "baseline", "tools/suppression-baseline.tsv")?,
    )?;
    let excludes_file = config_path(
        CHECK,
        &policy.string(CHECK, "excludes", "tools/suppression-ban-excludes")?,
    )?;
    let excludes = patterns::load_excludes(ctx, CHECK, &excludes_file)?;

    let mut out = Outcome::default();
    blanket_lanes(ctx, &excludes, &mut out)?;
    let blanket = out.violations;

    let counts = bare_counts(ctx, &excludes)?;
    // --update reads and rewrites the worktree copy — it is editing the
    // file the user will commit; the check reads the index like every
    // other policy input.
    let mut baseline = match update {
        true => baseline::load_worktree_baseline(ctx, CHECK, &baseline_file)?,
        false => baseline::load_baseline(ctx, CHECK, &baseline_file)?,
    };
    if update {
        baseline = tighten(ctx, &baseline_file, &baseline, &counts, &mut out)?;
    }
    ratchet(&baseline, &counts, &baseline_file, &mut out);
    let total = out.violations;
    match total {
        0 => out.say(format!(
            "suppression-ban: OK — no blanket suppressions, bare allows within baseline {baseline_file}"
        )),
        _ => out.say(format!(
            "suppression-ban: {total} violation(s) — {blanket} blanket, {} ratchet (baseline {baseline_file})",
            total - blanket
        )),
    }
    Ok(out)
}
