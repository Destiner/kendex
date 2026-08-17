//! Recorded decisions about single findings: "this one is not a problem",
//! written down so it stops asking.
//!
//! An acceptance (`overrides.rs`) is one decision about a whole item — read
//! everything, install anyway. A dismissal is smaller: one finding, on one
//! installation, judged not to be the problem the rule says it is. It never
//! unblocks anything; it only settles a question. Like an acceptance it
//! binds to the complete bytes that were reviewed and the rule set that
//! judged them, and the moment either moves it stops applying. Nothing here
//! can grow into a rule-level mute: a dismissal names one finding on one
//! content, and a different finding or a different content is a different
//! question.
//!
//! One snapshot per installation holds the proof once — the review hash and
//! the rule set — and every dismissal for that installation sits beneath it.
//! A dismissal made against newer content replaces the snapshot, taking the
//! older dismissals with it: they spoke for bytes that are gone.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::RULESET_VERSION;

/// Why a finding was dismissed. Every reason is a claim about the content
/// — a project's dismissals travel with the repository, so a reason must
/// mean the same thing to whoever reads it next, and none of them may be
/// one person's tolerance for risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum DismissReason {
    /// The rule misread this: nothing here does what the finding says.
    WrongCall,
    /// The flagged behaviour is what this item is for.
    Intended,
    /// The content is from a source the reviewer trusts. Bound to that
    /// source's identity: the same bytes from somewhere else are a
    /// different question.
    TrustedSource,
}

impl DismissReason {
    pub const ALL: [DismissReason; 3] = [
        DismissReason::WrongCall,
        DismissReason::Intended,
        DismissReason::TrustedSource,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DismissReason::WrongCall => "wrong-call",
            DismissReason::Intended => "intended",
            DismissReason::TrustedSource => "trusted-source",
        }
    }

    pub fn parse(value: &str) -> Option<DismissReason> {
        DismissReason::ALL
            .into_iter()
            .find(|reason| reason.name() == value)
    }
}

/// One dismissed finding, keyed in its snapshot by the finding's
/// fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct Dismissal {
    pub reason: DismissReason,
    pub dismissed_at: String,
    /// The source identity a `TrustedSource` dismissal trusted — the
    /// resolved provenance the lock records, or the git origin of an
    /// unmanaged item's files. Absent for the other reasons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Every dismissal for one installation, and the content they were made
/// against. Stored under the installation's key: kind, name and harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct SafetyReview {
    pub review_hash: String,
    pub ruleset: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dismissed: BTreeMap<String, Dismissal>,
}

impl SafetyReview {
    /// A fresh snapshot of this content with nothing decided under it yet.
    pub fn of(review_hash: &str) -> SafetyReview {
        SafetyReview {
            review_hash: review_hash.to_owned(),
            ruleset: RULESET_VERSION,
            dismissed: BTreeMap::new(),
        }
    }

    /// Whether this snapshot still describes the content in front of us:
    /// the same bytes, judged by the same rules. `None` for the hash means
    /// the bytes cannot be read here, and a decision with nothing to compare
    /// against never applies.
    pub fn stale_why(&self, review_hash: Option<&str>) -> Option<String> {
        let Some(review_hash) = review_hash else {
            return Some("the content it was made for cannot be read here, so nothing proves it is still what was reviewed".to_owned());
        };
        if self.review_hash != review_hash {
            return Some("the content changed since it was reviewed".to_owned());
        }
        if self.ruleset != RULESET_VERSION {
            return Some(format!(
                "the safety rules changed since it was reviewed (reviewed under rule set {}, now {RULESET_VERSION})",
                self.ruleset
            ));
        }
        None
    }
}

/// What a recorded dismissal means for the finding in front of us now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DismissalState {
    /// Still describing exactly this finding on exactly this content.
    Active,
    /// Recorded, but what it was made against has changed since.
    Stale { why: String },
}

/// Whether one dismissal still speaks for a finding. The snapshot answers
/// for the bytes and the rules; a `TrustedSource` dismissal additionally
/// answers for where the bytes came from, since the same bytes rebound to
/// another source were not what was trusted.
pub fn dismissal_state(
    review: &SafetyReview,
    dismissal: &Dismissal,
    review_hash: Option<&str>,
    provenance: Option<&str>,
) -> DismissalState {
    if let Some(why) = review.stale_why(review_hash) {
        return DismissalState::Stale { why };
    }
    if dismissal.reason != DismissReason::TrustedSource {
        return DismissalState::Active;
    }
    let Some(trusted) = dismissal.source.as_deref() else {
        return DismissalState::Stale {
            why: "the record does not say which source was trusted".to_owned(),
        };
    };
    match provenance {
        Some(current) if current == trusted => DismissalState::Active,
        Some(current) => DismissalState::Stale {
            why: format!("it trusted {trusted}, and this content now comes from {current}"),
        },
        None => DismissalState::Stale {
            why: format!(
                "it trusted {trusted}, and nothing here says where this content comes from now"
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn review() -> SafetyReview {
        SafetyReview::of("hash-1")
    }

    fn dismissal(reason: DismissReason, source: Option<&str>) -> Dismissal {
        Dismissal {
            reason,
            dismissed_at: "2026-01-01T00:00:00Z".to_owned(),
            source: source.map(str::to_owned),
        }
    }

    #[test]
    fn a_dismissal_binds_to_the_bytes_and_the_rules() {
        let review = review();
        let plain = dismissal(DismissReason::WrongCall, None);
        assert_eq!(
            dismissal_state(&review, &plain, Some("hash-1"), None),
            DismissalState::Active
        );
        assert!(matches!(
            dismissal_state(&review, &plain, Some("hash-2"), None),
            DismissalState::Stale { .. }
        ));
        assert!(matches!(
            dismissal_state(&review, &plain, None, None),
            DismissalState::Stale { .. }
        ));
        let old_rules = SafetyReview {
            ruleset: RULESET_VERSION + 1,
            ..review
        };
        assert!(matches!(
            dismissal_state(&old_rules, &plain, Some("hash-1"), None),
            DismissalState::Stale { .. }
        ));
    }

    /// Trusting a source is trusting *that* source: rebinding the same bytes
    /// elsewhere, or losing track of where they came from, ends it.
    #[test]
    fn a_trusted_source_dismissal_binds_to_the_source() {
        let review = review();
        let trusted = dismissal(DismissReason::TrustedSource, Some("owner/repo"));
        assert_eq!(
            dismissal_state(&review, &trusted, Some("hash-1"), Some("owner/repo")),
            DismissalState::Active
        );
        assert!(matches!(
            dismissal_state(&review, &trusted, Some("hash-1"), Some("local")),
            DismissalState::Stale { .. }
        ));
        assert!(matches!(
            dismissal_state(&review, &trusted, Some("hash-1"), None),
            DismissalState::Stale { .. }
        ));
        let unsaid = dismissal(DismissReason::TrustedSource, None);
        assert!(matches!(
            dismissal_state(&review, &unsaid, Some("hash-1"), Some("owner/repo")),
            DismissalState::Stale { .. }
        ));
    }

    #[test]
    fn reasons_round_trip_by_name() {
        for reason in DismissReason::ALL {
            assert_eq!(DismissReason::parse(reason.name()), Some(reason));
        }
        assert_eq!(DismissReason::parse("because"), None);
    }
}
