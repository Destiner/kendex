//! Safety scoring and the gate it feeds.
//!
//! `100 − Σ deductions`, floor 0. The first finding from a rule costs its
//! full severity; every repeat of that rule costs one point, so a file with
//! forty `curl | sh` lines does not score the same as one with a single
//! line, and neither does a single mistake bury the score under its own
//! echo. Every deduction names the rule and the place it fired.
//!
//! Repeats stop counting once they have cost as much as the first hit did.
//! Past that point they have said all they can say — that the pattern is
//! pervasive rather than incidental — and the real catalog is what settled
//! it: the kendex `github` skill reads `.env.local` on forty lines because
//! that is the skill's entire job, and an uncapped tail turned a Medium
//! finding into a blocked install.
//!
//! The aggregate warns; it does not decide alone. Threshold arithmetic on
//! its own lets one Critical finding through at 75, which is why any
//! Critical blocks by itself regardless of what the total says.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::{Finding, Severity};

/// One rule firing once, and what it cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Deduction {
    pub rule: String,
    pub location: String,
    pub severity: Severity,
    pub points: u32,
    /// A later hit from a rule that has already been counted in full.
    pub repeat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SafetyScore {
    pub score: u32,
    pub deductions: Vec<Deduction>,
}

/// Where warning starts and where installing stops. Configured in app
/// settings rather than in a manifest: a manifest travels with the
/// repository it describes, and a catalog that could lower the bar it is
/// measured against is not being measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct Thresholds {
    pub warn_below: u32,
    pub block_below: u32,
}

impl Default for Thresholds {
    fn default() -> Thresholds {
        Thresholds {
            warn_below: 80,
            block_below: 60,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    /// Nothing found worth saying.
    Clean,
    /// Findings worth reading before installing; the install proceeds.
    Warn,
    /// Nothing installs until a person reviews it.
    Block,
}

pub fn safety(findings: &[Finding]) -> SafetyScore {
    // Per rule: what the first hit cost, and what its repeats have cost so
    // far. Findings arrive worst-first, so the first hit sets the cap.
    let mut spent: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
    let mut score: i64 = 100;
    let mut deductions = Vec::new();
    for finding in findings {
        let full = finding.severity.deduction();
        let (repeat, points) = match spent.get_mut(finding.rule.as_str()) {
            None => {
                spent.insert(finding.rule.as_str(), (full, 0));
                (false, full)
            }
            Some((cap, repeats)) => {
                let point = u32::from(*repeats < *cap);
                *repeats += point;
                (true, point)
            }
        };
        score -= i64::from(points);
        deductions.push(Deduction {
            rule: finding.rule.clone(),
            location: finding.location.clone(),
            severity: finding.severity,
            points,
            repeat,
        });
    }
    SafetyScore {
        score: score.max(0).unsigned_abs() as u32,
        deductions,
    }
}

/// The verdict and the sentences that justify it. A Critical finding blocks
/// on its own; the aggregate blocks below its threshold and warns below the
/// higher one.
pub fn verdict(
    findings: &[Finding],
    score: &SafetyScore,
    thresholds: Thresholds,
) -> (Verdict, Vec<String>) {
    let mut reasons = Vec::new();
    for finding in findings.iter().filter(|f| f.severity == Severity::Critical) {
        reasons.push(format!(
            "{} at {} — {}",
            finding.rule, finding.location, finding.message
        ));
    }
    let blocked_by_score = score.score < thresholds.block_below;
    if blocked_by_score {
        reasons.push(format!(
            "safety score {} is below {}",
            score.score, thresholds.block_below
        ));
    }
    if !reasons.is_empty() {
        return (Verdict::Block, reasons);
    }
    if score.score < thresholds.warn_below {
        return (
            Verdict::Warn,
            vec![format!(
                "safety score {} is below {}",
                score.score, thresholds.warn_below
            )],
        );
    }
    match findings.is_empty() {
        true => (Verdict::Clean, Vec::new()),
        false => (
            Verdict::Warn,
            findings
                .iter()
                .map(|f| format!("{} at {} — {}", f.rule, f.location, f.message))
                .collect(),
        ),
    }
}
