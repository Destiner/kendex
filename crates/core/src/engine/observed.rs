//! Scoring what is on disk, as opposed to what a plan would write.

use crate::env::Env;
use crate::error::Result;
use crate::manifest::{self, Manifest, ManifestFile};
use crate::model::Scope;

use super::gate::{self, ItemSafety};

/// The other scoring path: the safety of what is on disk in this scope
/// right now, declared or not. The plan-time path scores content nobody has
/// installed yet, which is what gates a fresh install; this scores what a
/// tool would load if it started this second, which is what an audit is
/// about. Same rules, different bytes.
pub fn observed_safety(env: &Env, scope: &Scope) -> Result<Vec<ItemSafety>> {
    Ok(observed_rows(env, scope)?
        .into_iter()
        .filter(|row| {
            !row.findings.is_empty()
                || !row.skipped.is_empty()
                || row.verdict != crate::quality::Verdict::Clean
        })
        .collect())
}

/// Every installation in this scope, scored — the clean ones included. The
/// decision registry reads this: a recorded decision about an item that is
/// still installed but no longer has the finding is stale, not obsolete,
/// and telling those apart needs the clean rows too.
pub fn observed_rows(env: &Env, scope: &Scope) -> Result<Vec<ItemSafety>> {
    let scope = scope.canonical();
    let settings = crate::settings::load(env)?;
    let scan = crate::scan::scan_scopes(env, &settings.harness_roots, std::slice::from_ref(&scope));
    // The decisions recorded for this scope. An item that is installed
    // *because* someone read its findings and accepted them is not the same
    // thing as one nobody has looked at, and an audit that calls the first
    // one held back is telling the user the opposite of the truth.
    let manifest = match manifest::load(&manifest::manifest_path(env, &scope))? {
        ManifestFile::Current(manifest) => *manifest,
        _ => Manifest::default(),
    };
    // A lock this build cannot read is a note on the audit page, not a
    // reason to score nothing: the bytes on disk are still what a tool
    // would load. Without it, provenance falls back to the scanner's word.
    let lock = match crate::lock::load_file(&crate::lock::lock_path(env, &scope))? {
        crate::lock::LockFile::Current(lock) => lock,
        _ => crate::lock::Lock::default(),
    };
    let mut cache = crate::quality::observe::AuditCache::default();
    Ok(scan
        .items
        .iter()
        .map(|item| {
            let scored = crate::quality::observe::audit_observed(
                &mut cache,
                item,
                gate::content_hash,
                super::review_hash::observed,
            );
            let result = scored.result;
            let (verdict, reasons) =
                crate::quality::verdict(&result.findings, &result.safety, settings.safety);
            let key = crate::lock::entry_key(item.kind, &item.name, item.harness);
            let root = item.path.display().to_string();
            // Only the lock's word on where the bytes came from: what vstack
            // itself declared and resolved. The scanner's guess is a remote
            // url read out of a `.git/config` sitting inside the very
            // content being judged, which is not something to trust a
            // source by.
            let provenance = lock
                .entries
                .get(&key)
                .map(|entry| entry.source_repo.clone());
            let override_state = crate::quality::overrides::state(
                manifest.safety_overrides.get(&key),
                scored.review.as_deref(),
                &result.findings,
                &root,
            );
            let decisions = super::decisions::decisions(
                &super::decisions::Installation {
                    manifest: &manifest,
                    scope: &scope,
                    key: &key,
                    root: &root,
                    review_hash: scored.review.as_deref(),
                    provenance: provenance.as_deref(),
                    override_state: &override_state,
                    held_back: verdict == crate::quality::Verdict::Block
                        && !override_state.unblocks(),
                },
                &result.findings,
            );
            ItemSafety {
                kind: item.kind,
                name: item.name.clone(),
                harness: item.harness,
                scope: item.scope.clone(),
                location: root,
                safety: result.safety,
                quality: result.quality,
                override_state,
                findings: result.findings,
                skipped: result.skipped,
                verdict,
                reasons,
                content_hash: scored.content,
                review_hash: scored.review,
                provenance,
                decisions,
            }
        })
        .collect())
}
