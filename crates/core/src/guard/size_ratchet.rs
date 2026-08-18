//! size-ratchet — tighten-only file-size gate over tracked files, counted
//! from the index (the blobs a commit records). A path's threshold is the
//! first matching class rule, else the default; files over it must be
//! frozen in the baseline at their current size, and the baseline only
//! moves down: a loose or stale row is itself a failure, never slack.

use std::collections::BTreeMap;

use crate::error::Result;

use super::settings::{Policy, config_path};
use super::{GuardCtx, Outcome, baseline, guard_err, patterns};

const CHECK: &str = "size-ratchet";

pub enum Mode {
    Check,
    /// Write the first baseline from the gate's own collector; refuses if
    /// one exists — a seed must never quietly replace a reviewed ratchet.
    Seed,
    /// Tighten only: lower rows to current reality, remove rows for files
    /// now at/under their threshold or no longer counted; never add a
    /// row, never raise a number.
    Update,
}

/// One path's threshold: the first class rule it matches, else the
/// default. Ordering is load-bearing, which is why classes are an array.
struct Thresholds {
    default: u64,
    classes: Vec<ClassRule>,
}

struct ClassRule {
    pattern: String,
    threshold: u64,
    dialect: patterns::Dialect,
}

impl Thresholds {
    fn for_path(&self, path: &str) -> u64 {
        self.classes
            .iter()
            .find(|rule| patterns::matches(&rule.pattern, path, rule.dialect))
            .map(|rule| rule.threshold)
            .unwrap_or(self.default)
    }
}

fn thresholds(policy: &Policy) -> Result<Thresholds> {
    let default = policy.positive_int(CHECK, "threshold", 400)?;
    let legacy = policy.string(CHECK, "classes-dialect", "")? == "legacy-glob";
    let classes = policy
        .classes(CHECK)?
        .into_iter()
        .map(|(pattern, threshold)| ClassRule {
            pattern,
            threshold,
            dialect: match legacy {
                true => patterns::Dialect::LegacyGlob,
                false => patterns::Dialect::Gitignore,
            },
        })
        .collect();
    Ok(Thresholds { default, classes })
}

/// Per-file line counts over every tracked, non-excluded text blob: `git
/// grep -c` with a match-every-line pattern, one subprocess. Binary blobs
/// are skipped (`-I`) — their growth is the byte-ceiling's question. A
/// path the baseline TSV cannot carry (a tab or newline in the name) is
/// refused after excludes apply, so the refusal's own remedy — exclude it
/// — works.
fn line_counts(ctx: &GuardCtx, excludes: &patterns::Excludes) -> Result<BTreeMap<String, u64>> {
    let mut counts = ctx.grep_counts(CHECK, "^", &[])?;
    counts.retain(|path, _| !excludes.is_excluded(path));
    baseline::assert_baseline_representable(CHECK, &counts)?;
    Ok(counts)
}

fn evaluate(
    counts: &BTreeMap<String, u64>,
    baseline: &BTreeMap<String, u64>,
    thresholds: &Thresholds,
    baseline_file: &str,
    out: &mut Outcome,
) {
    for (path, count) in counts {
        let threshold = thresholds.for_path(path);
        match baseline.get(path) {
            None if *count > threshold => out.violation(
                format!(
                    "size-ratchet FAIL new offender: {path} — {count} lines > threshold {threshold}, no baseline row"
                ),
                "split the file, don't compress it; freezing a legacy size is a hand-added baseline row in this diff with justification",
            ),
            None => {}
            Some(frozen) if *count > *frozen => out.violation(
                format!("size-ratchet FAIL grew: {path} — {count} lines > baseline {frozen}"),
                "split the file back under its baseline; raising the row is a hand-edit in this diff with justification",
            ),
            Some(frozen) if *count <= threshold => out.violation(
                format!(
                    "size-ratchet FAIL stale baseline row: {path} — now {count} lines, at/under threshold {threshold}; the row ({frozen}) must go"
                ),
                &format!("run size-ratchet --update to drop the row from {baseline_file}"),
            ),
            Some(frozen) if *count < *frozen => out.violation(
                format!(
                    "size-ratchet FAIL baseline looser than reality: {path} — baseline {frozen} > actual {count}; the ratchet only moves down"
                ),
                &format!("run size-ratchet --update to tighten the row in {baseline_file}"),
            ),
            Some(_) => {}
        }
    }
    for (path, frozen) in baseline {
        if !counts.contains_key(path) {
            out.violation(
                format!(
                    "size-ratchet FAIL stale baseline row: {path} — the file left the tracked, non-excluded set; the row ({frozen}) must go"
                ),
                &format!("run size-ratchet --update to drop the row from {baseline_file}"),
            );
        }
    }
}

/// The baseline is a tracked file too, and `--update` is about to change
/// its length: its own row must describe the file it is about to become,
/// or the next run finds it looser than reality and needs a second pass
/// to settle. Rows are lines, so the rendered length is the row count;
/// the verdict below judges the file at that length.
fn reconcile_own_row(
    baseline_file: &str,
    thresholds: &Thresholds,
    updated: &mut BTreeMap<String, u64>,
    counts: &mut BTreeMap<String, u64>,
    out: &mut Outcome,
) {
    let Some(frozen) = updated.get(baseline_file).copied() else {
        return;
    };
    let length = updated.len() as u64;
    if length <= thresholds.for_path(baseline_file) {
        updated.remove(baseline_file);
        out.say(format!(
            "removed: {baseline_file} (now at/under its threshold)"
        ));
    } else if length < frozen {
        updated.insert(baseline_file.to_owned(), length);
        out.say(format!("tightened: {baseline_file} {frozen} -> {length}"));
    }
    counts.insert(baseline_file.to_owned(), updated.len() as u64);
}

pub fn run(ctx: &GuardCtx, policy: &Policy, mode: Mode) -> Result<Outcome> {
    let thresholds = thresholds(policy)?;
    let baseline_file = config_path(
        CHECK,
        &policy.string(CHECK, "baseline", "tools/size-ratchet-baseline.tsv")?,
    )?;
    let excludes_file = config_path(
        CHECK,
        &policy.string(CHECK, "excludes", "tools/size-ratchet-excludes")?,
    )?;
    let excludes = patterns::load_excludes(ctx, CHECK, &excludes_file)?;
    let mut counts = line_counts(ctx, &excludes)?;

    let mut out = Outcome::default();
    let target = ctx.root.join(&baseline_file);
    let baseline = match mode {
        Mode::Seed => {
            // A reviewed ratchet lives in HEAD as much as in the index or
            // on disk: staging its deletion must not unlock a re-seed that
            // recreates every row enlarged.
            let existing = target.is_file()
                || super::settings::policy_content(ctx, CHECK, &baseline_file)?.is_some()
                || ctx.head_has(CHECK, &baseline_file)?;
            if existing {
                return Err(guard_err(
                    CHECK,
                    format!(
                        "--seed refuses: a baseline already exists at {baseline_file}; tighten it with --update instead"
                    ),
                ));
            }
            let seeded: BTreeMap<String, u64> = counts
                .iter()
                .filter(|(path, count)| **count > thresholds.for_path(path))
                .map(|(path, count)| (path.clone(), *count))
                .collect();
            crate::fs::atomic_write(&target, &baseline::render_baseline(&seeded))?;
            out.say(format!(
                "size-ratchet --seed: baseline written at {baseline_file} ({} row(s))",
                seeded.len()
            ));
            seeded
        }
        Mode::Update => {
            let current = baseline::load_worktree_baseline(ctx, CHECK, &baseline_file)?;
            let mut updated = baseline::tighten(
                &current,
                &counts,
                |path, count| {
                    (count <= thresholds.for_path(path))
                        .then(|| "now at/under its threshold".to_owned())
                },
                &mut out,
            );
            reconcile_own_row(
                &baseline_file,
                &thresholds,
                &mut updated,
                &mut counts,
                &mut out,
            );
            if target.is_file() || !updated.is_empty() {
                crate::fs::atomic_write(&target, &baseline::render_baseline(&updated))?;
                out.say(format!(
                    "size-ratchet --update: baseline tightened at {baseline_file} ({} row(s))",
                    updated.len()
                ));
            }
            updated
        }
        Mode::Check => baseline::load_baseline(ctx, CHECK, &baseline_file)?,
    };

    evaluate(&counts, &baseline, &thresholds, &baseline_file, &mut out);
    let (checked, violations) = (counts.len(), out.violations);
    match violations {
        0 => out.say(format!(
            "size-ratchet: OK — {checked} tracked file(s) checked, default threshold {}",
            thresholds.default
        )),
        n => out.say(format!(
            "size-ratchet: {n} violation(s) — {checked} tracked file(s) checked (baseline {baseline_file})"
        )),
    }
    Ok(out)
}
