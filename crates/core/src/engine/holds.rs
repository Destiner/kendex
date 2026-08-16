//! The holds that outrank planning: situations where the right plan for
//! an item is to write nothing at all and say why — a revision wanted two
//! ways at once, or bytes the user's hands were on.

use std::collections::BTreeSet;

use super::desired::{Artifact, Desired};
use super::item_plan::PlanSink;
use super::{DriftCause, DriftRow, DriftState};
use crate::lock::Lock;
use crate::model::Scope;

/// An item wanted at two revisions at once writes nothing: the conflict
/// row says so, the existing install and its record stay exactly as they
/// were, and the expansion's warning already names the fix. Returns true
/// when the item was held back this way.
pub(super) fn hold_rev_conflict(
    item: &Desired,
    scope: &Scope,
    lock: &Lock,
    conflicts: &BTreeSet<(crate::model::ItemKind, String)>,
    sink: &mut PlanSink,
) -> bool {
    if !conflicts.contains(&(item.kind, item.name.clone())) {
        return false;
    }
    sink.drift.push(DriftRow {
        kind: item.kind,
        name: item.name.clone(),
        harness: item.harness,
        scope: scope.clone(),
        state: DriftState::Conflict,
        detail: "wanted at two different revisions — nothing was changed".into(),
        cause: None,
    });
    if let Some(entry) = lock.entries.get(&item.key) {
        sink.new_lock
            .entries
            .insert(item.key.clone(), entry.clone());
    }
    true
}

/// What the artifact's bytes on disk hash to right now — `None` when there
/// is nothing comparable (absent, a symlink where content should be, a
/// registration, unreadable). `None` never blocks: the paths that need a
/// human already produce conflicts of their own.
fn observed_artifact_hash(artifact: &Artifact) -> Option<String> {
    let path = match artifact {
        Artifact::File { path, .. } => path,
        Artifact::Tree { canonical, .. } => canonical,
        Artifact::Registration { .. } => return None,
    };
    if path.is_symlink() || !path.exists() {
        return None;
    }
    crate::hash::hash_tree(path).ok()
}

/// The user's hands were on this installation: hold it. An edited artifact
/// becomes a conflict naming the ways out — keep it as a fork, or discard
/// the edits — and no write op is planned for it. Returns true when the
/// item was held.
///
/// The classification is rendered-hash-first: what apply last wrote is the
/// anchor that tells an upstream move from a local edit. Disk matching the
/// desired bytes is never an edit, however it got there. An entry from
/// before the anchor existed falls back on the installation hash — inputs
/// unchanged means the desired render equals the install-time render, so a
/// differing disk is an edit — and when the inputs moved too, the honest
/// answer is that the two cannot be told apart, which is a conflict, never
/// an overwrite.
pub(super) fn hold_local_edit(
    item: &Desired,
    scope: &Scope,
    lock: &Lock,
    sink: &mut PlanSink,
) -> bool {
    let Some(entry) = lock.entries.get(&item.key) else {
        return false;
    };
    let Some(disk) = observed_artifact_hash(&item.artifact) else {
        return false;
    };
    let wanted = super::desired::artifact_disk_hash(&item.artifact);
    if disk == wanted {
        return false;
    }
    // Bytes some apply provably wrote are never an edit — whichever entry
    // wrote them. Trees change hands between passes (a command's tree taken
    // over by a skill, a per-tool variant collapsing onto the shared one),
    // and the lock as a whole knows every render that ever landed.
    if lock
        .entries
        .values()
        .filter_map(|entry| entry.rendered_hash.as_ref())
        .any(|rendered| *rendered == disk)
    {
        return false;
    }
    let hash_moved = entry.source_hash != item.hash;
    let cause = match (&entry.rendered_hash, hash_moved) {
        (Some(_), true) => DriftCause::Both,
        (Some(_), false) => DriftCause::LocalEdit,
        (None, false) => DriftCause::LocalEdit,
        (None, true) => DriftCause::Both,
    };
    let detail = match (cause, &entry.rendered_hash) {
        (DriftCause::Both, None) => {
            "changed upstream and on disk — vstack cannot tell your edits from the update; keep it as a fork or apply with edits discarded"
        }
        (DriftCause::Both, _) => {
            "edited on disk and changed upstream — keep your edits as a fork, or apply with edits discarded"
        }
        _ => "edited on disk since install — keep it as a fork, or apply with edits discarded",
    };
    sink.drift.push(DriftRow {
        kind: item.kind,
        name: item.name.clone(),
        harness: item.harness,
        scope: scope.clone(),
        state: DriftState::Conflict,
        detail: detail.into(),
        cause: Some(cause),
    });
    sink.new_lock
        .entries
        .insert(item.key.clone(), entry.clone());
    true
}
