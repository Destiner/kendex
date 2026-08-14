use serde::Serialize;
use specta::Type;
use vstack_core::engine::{self, DriftRow, ItemSafety, ItemWarning, PlanOptions, ops};
use vstack_core::env::Env;
use vstack_core::error::CoreError;
use vstack_core::model::{HarnessId, ItemKind, Scope};
use vstack_core::{apply, manifest};

fn env() -> Result<Env, String> {
    Env::detect().map_err(|e| e.to_string())
}

/// What the Audit page renders: drift rows plus the human-readable plan
/// that would fix them.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AuditView {
    pub scope: Scope,
    pub drift: Vec<DriftRow>,
    pub plan: Vec<String>,
    pub notes: Vec<String>,
    pub warnings: Vec<ItemWarning>,
    /// What the safety rules found in the content installed here. Each row
    /// carries two scores that are never combined: safety, which can hold an
    /// install back, and quality, which only ever informs.
    pub safety: Vec<ItemSafety>,
}

pub fn view(env: &Env, scope: &Scope) -> Result<AuditView, String> {
    let report = engine::audit(env, scope).map_err(|e| e.to_string())?;
    Ok(AuditView {
        scope: scope.clone(),
        drift: report.drift,
        plan: report
            .plan
            .ops
            .iter()
            .map(|op| op.description.clone())
            .collect(),
        notes: report.notes,
        warnings: report.warnings,
        safety: engine::observed_safety(env, scope).map_err(|e| e.to_string())?,
    })
}

#[tauri::command]
#[specta::specta]
pub fn audit_all() -> Result<Vec<AuditView>, String> {
    let env = env()?;
    let settings = vstack_core::settings::load(&env).map_err(|e| e.to_string())?;
    let mut scopes = vec![Scope::Global];
    scopes.extend(
        settings
            .projects
            .iter()
            .cloned()
            .map(|root| Scope::Project { root }),
    );
    scopes.iter().map(|scope| view(&env, scope)).collect()
}

/// The apply path plans through the same loader the audit view used, so
/// the listed plan is what executes — including the schema upgrade a v0.1
/// manifest is owed on its first apply. (Orphan removal is the one opt-in
/// extra; the dialog lists each left-behind item beside its checkbox.)
pub fn apply_scope(env: &Env, scope: &Scope, remove_orphans: bool) -> Result<AuditView, String> {
    // A manifest that vanished or turned legacy since the preview must be
    // said out loud, not answered with a silent empty apply.
    let path = manifest::manifest_path(env, scope);
    match manifest::load(&path).map_err(|e| e.to_string())? {
        manifest::ManifestFile::Current(_) => {}
        manifest::ManifestFile::Absent => return Err("no manifest for this scope yet".into()),
        manifest::ManifestFile::Legacy { .. } => {
            return Err(CoreError::LegacyManifest { path }.to_string());
        }
    }
    let options = PlanOptions {
        remove_orphans,
        removal_filter: None,
        ..PlanOptions::default()
    };
    let report = engine::plan_apply(env, scope, &options).map_err(|e| e.to_string())?;
    apply::execute(env, &report.plan, None).map_err(|e| e.to_string())?;
    view(env, scope)
}

#[tauri::command]
#[specta::specta]
pub fn apply_plan(scope: Scope, remove_orphans: bool) -> Result<AuditView, String> {
    apply_scope(&env()?, &scope, remove_orphans)
}

#[tauri::command]
#[specta::specta]
pub fn adopt_item(
    scope: Scope,
    kind: ItemKind,
    name: String,
    harness: HarnessId,
) -> Result<AuditView, String> {
    let env = env()?;
    let move_plan =
        engine::adopt::adopt(&env, &scope, kind, &name, harness).map_err(|e| e.to_string())?;
    apply::execute(&env, &move_plan, None).map_err(|e| e.to_string())?;
    let report = engine::audit(&env, &scope).map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    view(&env, &scope)
}

#[tauri::command]
#[specta::specta]
pub fn toggle_item(scope: Scope, name: String, enabled: bool) -> Result<AuditView, String> {
    let env = env()?;
    let report = ops::toggle(&env, &scope, std::slice::from_ref(&name), enabled)
        .map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    view(&env, &scope)
}

#[tauri::command]
#[specta::specta]
pub fn remove_item(scope: Scope, name: String) -> Result<AuditView, String> {
    let env = env()?;
    // Removing one item never takes its unneeded leftovers with it here:
    // the page has nowhere to preview that yet, and a sweep the user did
    // not see is exactly the surprise the preview step exists to stop.
    let report =
        ops::remove(&env, &scope, std::slice::from_ref(&name), false).map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    view(&env, &scope)
}
