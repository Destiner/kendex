use serde::Serialize;
use specta::Type;
use vstack_core::engine::{self, DriftRow, PlanOptions, ops};
use vstack_core::env::Env;
use vstack_core::lock::{load as load_lock, lock_path};
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
}

fn view(env: &Env, scope: &Scope) -> Result<AuditView, String> {
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

#[tauri::command]
#[specta::specta]
pub fn apply_plan(scope: Scope, remove_orphans: bool) -> Result<AuditView, String> {
    let env = env()?;
    let path = manifest::manifest_path(&env, &scope);
    let loaded = manifest::load_for_mutation(&path)
        .map_err(|e| e.to_string())?
        .ok_or("no manifest for this scope yet")?;
    let lock = load_lock(&lock_path(&env, &scope)).map_err(|e| e.to_string())?;
    let options = PlanOptions {
        remove_orphans,
        removal_filter: None,
    };
    let report =
        engine::plan_scope(&env, &scope, &loaded, &lock, &options).map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    view(&env, &scope)
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
    let report =
        ops::remove(&env, &scope, std::slice::from_ref(&name)).map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    view(&env, &scope)
}
