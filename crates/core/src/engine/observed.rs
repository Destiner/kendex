//! Scoring what is on disk, as opposed to what a plan would write.

use std::collections::BTreeMap;

use crate::env::Env;
use crate::error::Result;
use crate::manifest::{self, ManifestFile};
use crate::model::Scope;

use super::gate::{self, ItemSafety};

/// The other scoring path: the safety of what is on disk in this scope
/// right now, declared or not. The plan-time path scores content nobody has
/// installed yet, which is what gates a fresh install; this scores what a
/// tool would load if it started this second, which is what an audit is
/// about. Same rules, different bytes.
pub fn observed_safety(env: &Env, scope: &Scope) -> Result<Vec<ItemSafety>> {
    let scope = scope.canonical();
    let settings = crate::settings::load(env)?;
    let scan = crate::scan::scan_scopes(env, &settings.harness_roots, std::slice::from_ref(&scope));
    // The reviews recorded for this scope. An item that is installed
    // *because* someone read its findings and accepted them is not the same
    // thing as one nobody has looked at, and an audit that calls the first
    // one held back is telling the user the opposite of the truth.
    let overrides = match manifest::load(&manifest::manifest_path(env, &scope))? {
        ManifestFile::Current(manifest) => manifest.safety_overrides,
        _ => BTreeMap::new(),
    };
    let mut cache = crate::quality::observe::AuditCache::default();
    Ok(scan
        .items
        .iter()
        .map(|item| {
            let (content_hash, result) =
                crate::quality::observe::audit_observed(&mut cache, item, gate::content_hash);
            let (verdict, reasons) =
                crate::quality::verdict(&result.findings, &result.safety, settings.safety);
            let recorded =
                overrides.get(&crate::lock::entry_key(item.kind, &item.name, item.harness));
            ItemSafety {
                kind: item.kind,
                name: item.name.clone(),
                harness: item.harness,
                scope: item.scope.clone(),
                safety: result.safety,
                quality: result.quality,
                override_state: crate::quality::overrides::state(
                    recorded,
                    &content_hash,
                    &result.findings,
                ),
                findings: result.findings,
                skipped: result.skipped,
                verdict,
                reasons,
                content_hash,
            }
        })
        .filter(|row| {
            !row.findings.is_empty()
                || !row.skipped.is_empty()
                || row.verdict != crate::quality::Verdict::Clean
        })
        .collect())
}
