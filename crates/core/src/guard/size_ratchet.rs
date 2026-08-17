//! size-ratchet — tighten-only file-size gate over tracked files, counted
//! from the index (the blobs a commit records). A path's threshold is the
//! first matching class rule, else the default; files over it must be
//! frozen in the baseline at their current size, and the baseline only
//! moves down: a loose or stale row is itself a failure, never slack.

use std::collections::BTreeMap;

use crate::error::Result;

use super::settings::{Policy, config_path};
use super::{GuardCtx, Outcome, guard_err, patterns};

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

/// Per-file line counts over every tracked text blob, one subprocess:
/// `git grep -c` with a match-every-line pattern. Binary blobs are skipped
/// (`-I`) — their growth is the byte-ceiling's question. A file the
/// listing omits has no lines to gate.
fn line_counts(ctx: &GuardCtx) -> Result<BTreeMap<String, u64>> {
    let raw = ctx.git_grep(CHECK, &["grep", "--cached", "-cIzE", "^", "--"])?;
    let mut counts = BTreeMap::new();
    for record in raw.split(|byte| *byte == b'\n') {
        if record.is_empty() {
            continue;
        }
        let Some(nul) = record.iter().position(|byte| *byte == 0) else {
            return Err(guard_err(CHECK, "unparseable count record from git grep"));
        };
        let (path, count) = record.split_at(nul);
        let Ok(path) = std::str::from_utf8(path) else {
            return Err(guard_err(
                CHECK,
                format!(
                    "tracked path is not valid UTF-8 and cannot be represented in the baseline: {:?}",
                    String::from_utf8_lossy(path)
                ),
            ));
        };
        if path.contains('\t') {
            return Err(guard_err(
                CHECK,
                format!(
                    "tracked path contains a tab, unrepresentable in the baseline TSV (exclude it to skip the gate): '{path}'"
                ),
            ));
        }
        let count: u64 = std::str::from_utf8(&count[1..])
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .ok_or_else(|| guard_err(CHECK, format!("unparseable line count for '{path}'")))?;
        counts.insert(path.to_owned(), count);
    }
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

fn tighten(
    baseline: &BTreeMap<String, u64>,
    counts: &BTreeMap<String, u64>,
    thresholds: &Thresholds,
    out: &mut Outcome,
) -> BTreeMap<String, u64> {
    let mut updated = BTreeMap::new();
    for (path, frozen) in baseline {
        let Some(count) = counts.get(path) else {
            out.say(format!("removed: {path} (row {frozen})"));
            continue;
        };
        if *count <= thresholds.for_path(path) {
            out.say(format!("removed: {path} (now at/under its threshold)"));
            continue;
        }
        if count < frozen {
            out.say(format!("tightened: {path} {frozen} -> {count}"));
            updated.insert(path.clone(), *count);
        } else {
            if count > frozen {
                out.say(format!(
                    "kept (grew {count} > {frozen} — growth is a hand-edit, never --update): {path}"
                ));
            }
            updated.insert(path.clone(), *frozen);
        }
    }
    updated
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

    let mut counts = line_counts(ctx)?;
    counts.retain(|path, _| !excludes.is_excluded(path));

    let mut out = Outcome::default();
    let target = ctx.root.join(&baseline_file);
    let baseline = match mode {
        Mode::Seed => {
            let existing = target.is_file()
                || super::settings::policy_content(ctx, CHECK, &baseline_file)?.is_some();
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
            crate::fs::atomic_write(&target, &patterns::render_baseline(&seeded))?;
            out.say(format!(
                "size-ratchet --seed: baseline written at {baseline_file} ({} row(s))",
                seeded.len()
            ));
            seeded
        }
        Mode::Update => {
            let current = match target.is_file() {
                true => patterns::parse_baseline(
                    CHECK,
                    &baseline_file,
                    &std::fs::read_to_string(&target)
                        .map_err(|e| crate::error::CoreError::io(&target, e))?,
                )?,
                false => BTreeMap::new(),
            };
            let updated = tighten(&current, &counts, &thresholds, &mut out);
            if target.is_file() || !updated.is_empty() {
                crate::fs::atomic_write(&target, &patterns::render_baseline(&updated))?;
                out.say(format!(
                    "size-ratchet --update: baseline tightened at {baseline_file} ({} row(s))",
                    updated.len()
                ));
            }
            updated
        }
        Mode::Check => patterns::load_baseline(ctx, CHECK, &baseline_file)?,
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
