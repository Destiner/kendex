//! Recorded decisions to install something the gate blocked.
//!
//! An override is not permission to install an item; it is permission to
//! install *this content*, with *these findings*, judged by *these rules*.
//! It binds to all three plus the installation it was granted for, and the
//! moment any of them moves it stops applying and the block comes back.
//! Nothing here can grow into a standing exemption, which is the failure
//! mode every "allow this once" switch eventually has.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::{Finding, RULESET_VERSION};

/// One recorded review. The key it is stored under is the installation:
/// kind, name and harness, inside the scope whose manifest holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct SafetyOverride {
    /// Hash of the content that was reviewed.
    pub content_hash: String,
    /// The rule set that produced the findings below.
    pub ruleset: u32,
    /// Fingerprints of the exact findings that were reviewed, sorted.
    pub findings: Vec<String>,
    pub granted_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Whether a recorded override still speaks for what is in front of us.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum OverrideState {
    /// Nothing recorded for this installation.
    Absent,
    /// Recorded, and still describing exactly this.
    Active,
    /// Recorded, but what it was granted against has changed since.
    Stale { why: String },
}

impl OverrideState {
    pub fn unblocks(&self) -> bool {
        matches!(self, OverrideState::Active)
    }
}

/// The fingerprints of a finding set, in the one order two sets can be
/// compared in. `root` is the item's location, stripped from each print so
/// two readings of the same bytes at different paths compare equal.
pub fn fingerprints(findings: &[Finding], root: &str) -> Vec<String> {
    let mut prints: Vec<String> = findings.iter().map(|f| f.fingerprint(root)).collect();
    prints.sort();
    prints.dedup();
    prints
}

/// Record a review of exactly this content and these findings.
pub fn mint(
    content_hash: &str,
    findings: &[Finding],
    root: &str,
    note: Option<String>,
) -> SafetyOverride {
    SafetyOverride {
        content_hash: content_hash.to_owned(),
        ruleset: RULESET_VERSION,
        findings: fingerprints(findings, root),
        granted_at: crate::clock::timestamp(),
        note,
    }
}

/// What a recorded override means for the content in front of us now.
pub fn state(
    recorded: Option<&SafetyOverride>,
    content_hash: &str,
    findings: &[Finding],
    root: &str,
) -> OverrideState {
    let Some(recorded) = recorded else {
        return OverrideState::Absent;
    };
    if recorded.content_hash != content_hash {
        return OverrideState::Stale {
            why: "the content changed since it was reviewed".to_owned(),
        };
    }
    if recorded.ruleset != RULESET_VERSION {
        return OverrideState::Stale {
            why: format!(
                "the safety rules changed since it was reviewed (reviewed under rule set {}, now {RULESET_VERSION})",
                recorded.ruleset
            ),
        };
    }
    if recorded.findings != fingerprints(findings, root) {
        return OverrideState::Stale {
            why: "different problems were found than the ones that were reviewed".to_owned(),
        };
    }
    OverrideState::Active
}
