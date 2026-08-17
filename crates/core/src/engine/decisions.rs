//! What has been decided about each finding on an installation, and the
//! token a decision is made with.
//!
//! The rules produce findings; people produce decisions. The two are kept
//! apart — a `Finding` is a pure observation, built before anyone knows
//! which installation it belongs to — and joined here, once the installation,
//! its complete bytes and the records in its scope's manifest are all in
//! hand. Every finding gets a token naming exactly it on exactly this
//! content, and that token is the only thing a dismiss command accepts: the
//! UI never spells a decision key, so it can never spell the wrong one.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::manifest::Manifest;
use crate::quality::Finding;
use crate::quality::overrides::OverrideState;
use crate::quality::reviews::{DismissReason, DismissalState, dismissal_state};

use super::gate::SHOWN_HASH;

/// What is recorded about one finding, read against the content in front
/// of us now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum DecisionState {
    /// Nobody has ruled on this finding for this content. `earlier` says why
    /// a previous ruling no longer applies, when there was one.
    Open {
        #[serde(skip_serializing_if = "Option::is_none")]
        earlier: Option<String>,
    },
    /// Judged not to be a problem, for exactly this content.
    Dismissed {
        reason: DismissReason,
        dismissed_at: String,
    },
    /// Covered by an acceptance of the whole item: every finding on it was
    /// read and the item installed anyway.
    Accepted { granted_at: String },
}

impl DecisionState {
    /// Whether a person still has to look at this finding.
    pub fn is_open(&self) -> bool {
        matches!(self, DecisionState::Open { .. })
    }
}

/// One finding as a thing a person can rule on. Sits beside the finding it
/// is about — `ItemSafety.decisions[i]` speaks for `ItemSafety.findings[i]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FindingDecision {
    /// Names exactly this finding on exactly this content. Opaque to the
    /// UI; the only thing a dismiss command accepts. Absent where the
    /// content cannot be read here — there is nothing exact to bind a
    /// decision to, so none can be made.
    pub token: Option<String>,
    pub state: DecisionState,
}

/// The pieces a token binds: an installation, one finding on it, and the
/// review hash of the content it was found in. Spelled
/// `<kind:name:harness>#<fingerprint>@<review-hash>`; a hand-typed one may
/// carry a prefix of the hash, the same way `--allow-unsafe` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionToken {
    pub key: String,
    pub fingerprint: String,
    pub hash: String,
}

impl DecisionToken {
    pub fn parse(token: &str) -> Option<DecisionToken> {
        let (rest, hash) = token.rsplit_once('@')?;
        let (key, fingerprint) = rest.rsplit_once('#')?;
        if key.is_empty() || fingerprint.is_empty() || hash.len() < SHOWN_HASH {
            return None;
        }
        Some(DecisionToken {
            key: key.to_owned(),
            fingerprint: fingerprint.to_owned(),
            hash: hash.to_owned(),
        })
    }

    /// Whether this token names the content whose review hash this is.
    pub fn names(&self, review_hash: &str) -> bool {
        review_hash.starts_with(&self.hash)
    }
}

impl std::fmt::Display for DecisionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}@{}", self.key, self.fingerprint, self.hash)
    }
}

/// The token as it is printed for a person to type back: the hash cut to
/// the same length the accept flag uses.
pub fn short_token(key: &str, fingerprint: &str, review_hash: &str) -> String {
    DecisionToken {
        key: key.to_owned(),
        fingerprint: fingerprint.to_owned(),
        hash: review_hash[..SHOWN_HASH.min(review_hash.len())].to_owned(),
    }
    .to_string()
}

/// One decision per finding, in the findings' order.
///
/// An active acceptance of the item speaks for every finding on it. Below
/// that, a dismissal speaks for the one finding it names, for as long as the
/// snapshot it sits under still describes these bytes and, for a trusted
/// source, this provenance. Where the review hash is unknown no token can
/// be issued — there is nothing exact to bind a decision to — and every
/// finding is simply open.
pub fn decisions(
    manifest: &Manifest,
    key: &str,
    root: &str,
    review_hash: Option<&str>,
    provenance: Option<&str>,
    override_state: &OverrideState,
    findings: &[Finding],
) -> Vec<FindingDecision> {
    let review = manifest.safety_reviews.get(key);
    let accepted = manifest
        .safety_overrides
        .get(key)
        .filter(|_| override_state.unblocks());
    findings
        .iter()
        .map(|finding| {
            let fingerprint = finding.fingerprint(root);
            let token = review_hash.map(|hash| {
                DecisionToken {
                    key: key.to_owned(),
                    fingerprint: fingerprint.clone(),
                    hash: hash.to_owned(),
                }
                .to_string()
            });
            let dismissed = review.and_then(|r| r.dismissed.get(&fingerprint).map(|d| (r, d)));
            let state = match (accepted, dismissed) {
                (Some(recorded), _) => DecisionState::Accepted {
                    granted_at: recorded.granted_at.clone(),
                },
                (None, Some((review, dismissal))) => {
                    match dismissal_state(review, dismissal, review_hash, provenance) {
                        DismissalState::Active => DecisionState::Dismissed {
                            reason: dismissal.reason,
                            dismissed_at: dismissal.dismissed_at.clone(),
                        },
                        DismissalState::Stale { why } => DecisionState::Open { earlier: Some(why) },
                    }
                }
                (None, None) => DecisionState::Open { earlier: None },
            };
            FindingDecision { token, state }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_round_trips_and_a_short_hash_is_refused() {
        let token = DecisionToken {
            key: "plugin:chrome@openai-bundled:claude".to_owned(),
            fingerprint: "3fa9c2d1e0b4a7c8".to_owned(),
            hash: "abcdefabcdefabcdef".to_owned(),
        };
        assert_eq!(
            DecisionToken::parse(&token.to_string()),
            Some(token.clone())
        );
        assert!(token.names("abcdefabcdefabcdef0000"));
        assert!(!token.names("abcdefabcdefabcde"));
        assert!(DecisionToken::parse("plugin:x:claude#3fa9@abc").is_none());
        assert!(DecisionToken::parse("nothing").is_none());
    }
}
