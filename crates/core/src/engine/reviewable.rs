//! The one derivation of "what still needs a person" — the Review page,
//! the sidebar and Home counts, the scope summaries, the finished state,
//! and the drift snapshot all read this, so none of them can quote a
//! different number.
//!
//! Two things need a person: an install the gate is holding back (settled
//! by accepting or removing the item), and a finding on installed content
//! nobody has ruled on yet (settled by dismissing it). A dismissed or
//! accepted finding is not one of them, and neither is a held-back item's
//! individual finding — that item is counted once, as held back.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use specta::Type;

use super::decisions::DecisionState;
use super::gate::ItemSafety;
use crate::model::ItemKind;

/// What a scope still needs a person for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSummary {
    /// Installs the gate is holding back, counted once each.
    pub held_back: usize,
    /// Open findings counted once per distinct evidence — the same bytes
    /// carrying the same finding through several tools is one decision,
    /// because no rule reads the tool.
    pub open_evidence: usize,
}

impl ReviewSummary {
    pub fn needs_person(&self) -> usize {
        self.held_back + self.open_evidence
    }
}

/// The identity one piece of evidence is counted by: the bytes (review
/// hash, or the installation key where the bytes cannot be read here) and
/// the finding on them.
fn evidence_key(row: &ItemSafety, fingerprint: &str) -> (String, String) {
    let content = row
        .review_hash
        .clone()
        .unwrap_or_else(|| format!("{}:{}:{}", row.kind.name(), row.name, row.harness.name()));
    (content, fingerprint.to_owned())
}

/// The scope's whole count, from every scored row (clean ones included).
pub fn review_summary(rows: &[ItemSafety]) -> ReviewSummary {
    let held_back = rows.iter().filter(|row| row.blocked()).count();
    let mut evidence: BTreeSet<(String, String)> = BTreeSet::new();
    for row in rows.iter().filter(|row| !row.blocked()) {
        for decision in &row.decisions {
            if matches!(decision.state, DecisionState::Open { .. }) {
                evidence.insert(evidence_key(row, &decision.fingerprint));
            }
        }
    }
    ReviewSummary {
        held_back,
        open_evidence: evidence.len(),
    }
}

/// Open evidence per package — what the drift snapshot records beside each
/// package so the session check can say "N findings await review" without
/// re-scoring anything. A held-back item counts once, as itself.
pub fn open_by_package(rows: &[ItemSafety]) -> BTreeMap<(ItemKind, String), usize> {
    let mut evidence: BTreeMap<(ItemKind, String), BTreeSet<(String, String)>> = BTreeMap::new();
    let mut held: BTreeMap<(ItemKind, String), usize> = BTreeMap::new();
    for row in rows {
        let package = (row.kind, row.name.clone());
        if row.blocked() {
            // Counted once however many tools hold it: the decision is about
            // the item, and the package is the unit the report speaks in.
            held.insert(package, 1);
            continue;
        }
        for decision in &row.decisions {
            if matches!(decision.state, DecisionState::Open { .. }) {
                evidence
                    .entry(package.clone())
                    .or_default()
                    .insert(evidence_key(row, &decision.fingerprint));
            }
        }
    }
    let mut out: BTreeMap<(ItemKind, String), usize> = evidence
        .into_iter()
        .map(|(package, evidence)| (package, evidence.len()))
        .collect();
    for (package, count) in held {
        *out.entry(package).or_default() += count;
    }
    out
}

// These mirror ui/src/lib/reviewable.test.ts scenario for scenario: the
// two derivations must never disagree about what still needs a person.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HarnessId, Scope};
    use crate::quality::Verdict;
    use crate::quality::overrides::OverrideState;

    fn row(name: &str, harness: HarnessId, review: Option<&str>) -> ItemSafety {
        ItemSafety {
            kind: ItemKind::Skill,
            name: name.to_owned(),
            harness,
            scope: Scope::Global,
            location: format!("/tmp/{name}"),
            safety: crate::quality::SafetyScore {
                score: 100,
                deductions: Vec::new(),
            },
            quality: None,
            findings: Vec::new(),
            skipped: Vec::new(),
            verdict: Verdict::Warn,
            reasons: Vec::new(),
            content_hash: "content".into(),
            review_hash: review.map(str::to_owned),
            provenance: None,
            override_state: OverrideState::Absent,
            decisions: Vec::new(),
        }
    }

    fn decision(
        fingerprint: &str,
        state: DecisionState,
    ) -> super::super::decisions::FindingDecision {
        super::super::decisions::FindingDecision {
            fingerprint: fingerprint.to_owned(),
            token: None,
            state,
        }
    }

    fn open(fingerprint: &str) -> super::super::decisions::FindingDecision {
        decision(fingerprint, DecisionState::Open { earlier: None })
    }

    #[test]
    fn dismissed_and_accepted_findings_are_not_open() {
        let mut item = row("gh", HarnessId::Claude, Some("hash-a"));
        item.decisions = vec![
            open("f1"),
            decision(
                "f2",
                DecisionState::Dismissed {
                    reason: crate::quality::reviews::DismissReason::WrongCall,
                    dismissed_at: "now".into(),
                },
            ),
            decision(
                "f3",
                DecisionState::Accepted {
                    granted_at: "now".into(),
                },
            ),
        ];
        let summary = review_summary(&[item]);
        assert_eq!(summary.open_evidence, 1);
        assert_eq!(summary.held_back, 0);
        assert_eq!(summary.needs_person(), 1);
    }

    #[test]
    fn a_held_back_item_counts_once_never_per_finding() {
        let mut item = row("danger", HarnessId::Claude, Some("hash-a"));
        item.verdict = Verdict::Block;
        item.decisions = vec![open("f1"), open("f2"), open("f3")];
        let summary = review_summary(std::slice::from_ref(&item));
        assert_eq!(summary.held_back, 1);
        assert_eq!(summary.open_evidence, 0);

        // An accepted block is installed and staying — the opposite of held.
        item.override_state = OverrideState::Active;
        let summary = review_summary(&[item]);
        assert_eq!(summary.held_back, 0);
        assert_eq!(summary.open_evidence, 3);
    }

    #[test]
    fn same_bytes_same_finding_through_two_tools_is_one_evidence() {
        let mut claude = row("gh", HarnessId::Claude, Some("hash-a"));
        claude.decisions = vec![open("f1")];
        let mut codex = row("gh", HarnessId::Codex, Some("hash-a"));
        codex.decisions = vec![open("f1")];
        assert_eq!(
            review_summary(&[claude.clone(), codex.clone()]).open_evidence,
            1
        );
        assert_eq!(
            open_by_package(&[claude, codex]).get(&(ItemKind::Skill, "gh".into())),
            Some(&1)
        );
    }

    #[test]
    fn different_bytes_stay_separate_evidence_however_alike_the_finding() {
        let mut one = row("gh", HarnessId::Claude, Some("hash-a"));
        one.decisions = vec![open("f1")];
        let mut two = row("gh", HarnessId::Codex, Some("hash-b"));
        two.decisions = vec![open("f1")];
        assert_eq!(review_summary(&[one, two]).open_evidence, 2);
    }

    #[test]
    fn unreadable_content_falls_back_to_the_installation_identity() {
        let mut one = row("gh", HarnessId::Claude, None);
        one.decisions = vec![open("f1")];
        let mut two = row("gh", HarnessId::Codex, None);
        two.decisions = vec![open("f1")];
        // No review hash means no proof the bytes are the same: two items.
        assert_eq!(review_summary(&[one, two]).open_evidence, 2);
    }
}
