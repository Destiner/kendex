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

/// What a dismissal came back with: the scope's fresh view, and the exact
/// record written — an undo takes back this record and no newer one.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Dismissed {
    pub view: AuditView,
    pub dismissed_at: String,
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
    let dismissed_at = written_at(&env, &scope, &targets)?;
    Ok(Dismissed {
        view: view(&env, &scope),
        dismissed_at,
    })
}

/// The timestamp the write stamped on these records — read back from the
/// manifest, so the undo carries what is on disk rather than what the
/// caller thinks the clock said.
fn written_at(env: &Env, scope: &Scope, targets: &[DismissTarget]) -> Result<String, String> {
    let path = manifest::manifest_path(env, scope);
    let manifest::ManifestFile::Current(manifest) =
        manifest::load(&path).map_err(|e| e.to_string())?
    else {
        return Err("the manifest could not be read back after the write".to_owned());
    };
    let first = targets
        .first()
        .ok_or_else(|| "nothing to dismiss".to_owned())?;
    manifest
        .safety_reviews
        .get(&first.token.key)
        .and_then(|review| review.dismissed.get(&first.token.fingerprint))
        .map(|dismissal| dismissal.dismissed_at.clone())
        .ok_or_else(|| "the dismissal was not found after the write".to_owned())
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
