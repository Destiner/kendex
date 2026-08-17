//! Safety decisions from the app: dismissing a finding, and the registry of
//! every decision recorded so far.

use serde::Serialize;
use specta::Type;
use vstack_core::engine::ops::{self, DismissTarget, RecordedDecision};
use vstack_core::env::Env;
use vstack_core::model::Scope;
use vstack_core::quality::reviews::DismissReason;
use vstack_core::{apply, manifest};

use crate::audit::{AuditView, ScopeError, view};

fn env() -> Result<Env, String> {
    Env::detect().map_err(|e| e.to_string())
}

fn every_scope(env: &Env) -> Result<Vec<Scope>, String> {
    let settings = vstack_core::settings::load(env).map_err(|e| e.to_string())?;
    let mut scopes = vec![Scope::Global];
    scopes.extend(
        settings
            .projects
            .iter()
            .cloned()
            .map(|root| Scope::Project { root }),
    );
    Ok(scopes)
}

/// A scope whose decisions could not be read, carried as data beside the
/// ones that could. A view promising every decision must say which
/// scopes it is not speaking for, never silently skip them.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DecisionsScopeError {
    pub scope: Scope,
    pub error: ScopeError,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DecisionsView {
    pub decisions: Vec<RecordedDecision>,
    pub errors: Vec<DecisionsScopeError>,
}

/// Every recorded decision across every scope, each read against what is
/// installed there now.
#[tauri::command(async)]
#[specta::specta]
pub fn list_decisions() -> Result<DecisionsView, String> {
    decisions_view(&env()?)
}

pub fn decisions_view(env: &Env) -> Result<DecisionsView, String> {
    let mut decisions = Vec::new();
    let mut errors = Vec::new();
    for scope in every_scope(env)? {
        match ops::list_decisions(env, &scope) {
            Ok(mut listed) => decisions.append(&mut listed),
            Err(error) => errors.push(DecisionsScopeError {
                scope,
                error: ScopeError::from(&error),
            }),
        }
    }
    Ok(DecisionsView { decisions, errors })
}

/// One record a dismissal wrote, as an undo names it: the same key and
/// fingerprint the registry uses, and the timestamp that pins this exact
/// record so an old undo cannot delete a newer decision at the same key.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DismissedRecord {
    pub key: String,
    pub fingerprint: String,
    pub dismissed_at: String,
}

/// What a dismissal came back with: the scope's fresh view, and exactly
/// what was written.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Dismissed {
    pub view: AuditView,
    pub records: Vec<DismissedRecord>,
}

/// Dismiss the findings these tokens name, for one reason, in one scope.
/// The tokens are re-read against a fresh audit before anything is written;
/// one that no longer names what is installed stops the whole call.
#[tauri::command(async)]
#[specta::specta]
pub fn dismiss_findings(
    scope: Scope,
    tokens: Vec<String>,
    reason: DismissReason,
) -> Result<Dismissed, String> {
    let env = env()?;
    let targets = tokens
        .iter()
        .map(|token| DismissTarget::parse(token))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let plan = ops::dismiss(&env, &scope, &targets, reason).map_err(|e| e.to_string())?;
    apply::execute(&env, &plan, None).map_err(|e| e.to_string())?;
    let records = written(&env, &scope, &targets)?;
    Ok(Dismissed {
        view: view(&env, &scope),
        records,
    })
}

/// The records as the write left them — read back from the manifest, so an
/// undo carries what is on disk rather than what the caller thinks the
/// clock said.
fn written(
    env: &Env,
    scope: &Scope,
    targets: &[DismissTarget],
) -> Result<Vec<DismissedRecord>, String> {
    let path = manifest::manifest_path(env, scope);
    let manifest::ManifestFile::Current(manifest) =
        manifest::load(&path).map_err(|e| e.to_string())?
    else {
        return Err("the manifest could not be read back after the write".to_owned());
    };
    targets
        .iter()
        .map(|target| {
            manifest
                .safety_reviews
                .get(&target.token.key)
                .and_then(|review| review.dismissed.get(&target.token.fingerprint))
                .map(|dismissal| DismissedRecord {
                    key: target.token.key.clone(),
                    fingerprint: target.token.fingerprint.clone(),
                    dismissed_at: dismissal.dismissed_at.clone(),
                })
                .ok_or_else(|| "the dismissal was not found after the write".to_owned())
        })
        .collect()
}

/// Take a dismissal back. `dismissed_at` pins the exact record: a stale undo
/// finding a newer dismissal at the same key refuses rather than deleting
/// somebody's later decision.
#[tauri::command(async)]
#[specta::specta]
pub fn revoke_dismissal(
    scope: Scope,
    key: String,
    fingerprint: String,
    dismissed_at: String,
) -> Result<AuditView, String> {
    let env = env()?;
    let plan = ops::revoke_dismissal(&env, &scope, &key, &fingerprint, Some(&dismissed_at))
        .map_err(|e| e.to_string())?;
    apply::execute(&env, &plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}

#[tauri::command(async)]
#[specta::specta]
pub fn revoke_safety_override(scope: Scope, key: String) -> Result<AuditView, String> {
    let env = env()?;
    let plan = ops::revoke_override(&env, &scope, &key).map_err(|e| e.to_string())?;
    apply::execute(&env, &plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}
