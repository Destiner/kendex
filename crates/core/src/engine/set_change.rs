//! What a plan changes about the installed set, as opposed to what it
//! regenerates. Regenerating an installation that stays is safe to do
//! unasked — generated content is replaceable by construction — while
//! adding or dropping one is a decision, so it is previewed and confirmed.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::lock::{Lock, LockEntry, Reason};
use crate::model::{HarnessId, ItemKind, Scope};

/// Whether a plan brings an installation into being or takes one away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum SetDirection {
    Add,
    Remove,
}

/// One installation a plan adds or drops, as opposed to regenerating one
/// that stays. Regeneration is safe to do unasked — the content is
/// replaceable by construction — while changing *what is installed* is a
/// decision, so it is previewed and confirmed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetChange {
    pub kind: ItemKind,
    pub name: String,
    pub harness: HarnessId,
    pub direction: SetDirection,
    /// Why, in the words a preview shows.
    pub reason: String,
}

impl SetChange {
    pub(super) fn added(scope: &Scope, entry: &LockEntry) -> SetChange {
        SetChange {
            reason: why_wanted(scope, &entry.reasons),
            direction: SetDirection::Add,
            kind: entry.kind,
            name: entry.name.clone(),
            harness: entry.harness,
        }
    }

    pub(super) fn dropped(scope: &Scope, entry: &LockEntry) -> SetChange {
        let reason = match entry.reasons.contains(&Reason::Requested) {
            true => "no longer declared here".to_owned(),
            false => format!(
                "nothing needs it anymore — it was {}",
                why_wanted(scope, &entry.reasons)
            ),
        };
        SetChange {
            reason,
            direction: SetDirection::Remove,
            kind: entry.kind,
            name: entry.name.clone(),
            harness: entry.harness,
        }
    }
}

/// The reasons an installation exists, said once, in the words a preview
/// uses. Reasons from another scope name it — a global bundle pulling a
/// project item in has to read as what it is.
fn why_wanted(scope: &Scope, reasons: &BTreeSet<Reason>) -> String {
    let mut said: Vec<String> = Vec::new();
    for reason in reasons {
        let elsewhere = |other: &Scope| match other == scope {
            true => String::new(),
            false => format!(" in {}", other.label()),
        };
        said.push(match reason {
            Reason::Requested => "asked for".to_owned(),
            Reason::RequiredBy { by } => format!(
                "required by the {} {}{}",
                by.kind.name(),
                by.name,
                elsewhere(&by.scope)
            ),
            Reason::MemberOf { bundle } => {
                format!(
                    "part of the {} bundle{}",
                    bundle.name,
                    elsewhere(&bundle.scope)
                )
            }
        });
    }
    said.join(", and ")
}

/// The installed set before against the installed set after — every
/// installation this plan brings into being or takes away, whatever the
/// reason. Regeneration of an installation that stays is not in here.
pub(super) fn set_changes(scope: &Scope, before: &Lock, after: &Lock) -> Vec<SetChange> {
    let mut changes: Vec<SetChange> = after
        .entries
        .iter()
        .filter(|(key, _)| !before.entries.contains_key(*key))
        .map(|(_, entry)| SetChange::added(scope, entry))
        .collect();
    changes.extend(
        before
            .entries
            .iter()
            .filter(|(key, _)| !after.entries.contains_key(*key))
            .map(|(_, entry)| SetChange::dropped(scope, entry)),
    );
    changes
}
