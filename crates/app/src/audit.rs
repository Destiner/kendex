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

/// Why a scope couldn't be audited: a kind the UI can act on (retry, remove
/// the project, show the file) plus the plain-words message underneath it.
#[derive(Serialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum ScopeErrorKind {
    /// The lock exists but isn't readable as JSON, or as this build's lock
    /// shape — damaged, not merely old.
    LockCorrupt,
    /// The manifest or lock was written by a newer vstack than this one.
    SchemaTooNew,
    /// The manifest parses but fails validation.
    ManifestInvalid,
    Other,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScopeError {
    pub kind: ScopeErrorKind,
    pub message: String,
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
    /// Installations the plan would write but the safety gate holds back.
    /// Kept apart from `safety` (which scores what is on disk) because the
    /// two describe different bytes: an accept has to name the hash of what
    /// apply would write, and only these rows carry it.
    pub held_back: Vec<ItemSafety>,
    /// Set when this one scope couldn't be read at all — a corrupt or
    /// future-version lock or manifest. Carried as data so one scope's
    /// failure never blanks every other scope's audit (drift/plan/notes/
    /// warnings/safety are empty alongside it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ScopeError>,
}

impl AuditView {
    fn failed(scope: &Scope, error: &CoreError) -> Self {
        let kind = match error {
            CoreError::LockCorrupt { .. } => ScopeErrorKind::LockCorrupt,
            CoreError::SchemaTooNew { .. } => ScopeErrorKind::SchemaTooNew,
            CoreError::ManifestInvalid { .. } => ScopeErrorKind::ManifestInvalid,
            _ => ScopeErrorKind::Other,
        };
        AuditView {
            scope: scope.clone(),
            drift: Vec::new(),
            plan: Vec::new(),
            notes: Vec::new(),
            warnings: Vec::new(),
            safety: Vec::new(),
            held_back: Vec::new(),
            error: Some(ScopeError {
                kind,
                message: error.to_string(),
            }),
        }
    }
}

pub fn view(env: &Env, scope: &Scope) -> AuditView {
    let report = match engine::audit(env, scope) {
        Ok(report) => report,
        Err(e) => return AuditView::failed(scope, &e),
    };
    let safety = match engine::observed_safety(env, scope) {
        Ok(safety) => safety,
        Err(e) => return AuditView::failed(scope, &e),
    };
    AuditView {
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
        safety,
        held_back: report
            .safety
            .into_iter()
            .filter(|row| row.blocked())
            .collect(),
        error: None,
    }
}

#[tauri::command(async)]
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
    // One scope's unreadable lock or manifest must not blank the rest of the
    // audit — each scope's failure is carried as data on its own view.
    Ok(scopes.iter().map(|scope| view(&env, scope)).collect())
}

/// The apply path plans through the same loader the audit view used, so
/// the listed plan is what executes — including the schema upgrade a v0.1
/// manifest is owed on its first apply. (Orphan removal is the one opt-in
/// extra; the dialog lists each left-behind item beside its checkbox.)
pub fn apply_scope(
    env: &Env,
    scope: &Scope,
    remove_orphans: bool,
    allow_unsafe: Vec<String>,
) -> Result<AuditView, String> {
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
        allow_unsafe,
        ..PlanOptions::default()
    };
    let report = engine::plan_apply(env, scope, &options).map_err(|e| e.to_string())?;
    // An acceptance that no longer matches anything must stop the whole
    // apply, out loud. The engine ignores an unmatched token by design (a
    // stale flag must not grant), but a button that says "accept and
    // install" silently installing everything *except* the accepted item
    // would be worse than failing.
    for token in &options.allow_unsafe {
        let name = token.rsplit_once('@').map_or(token.as_str(), |(n, _)| n);
        let named: Vec<_> = report
            .safety
            .iter()
            .filter(|row| row.name == name)
            .collect();
        if named.is_empty() {
            return Err(format!(
                "nothing this apply would write is named '{name}' — nothing was changed"
            ));
        }
        if named.iter().any(|row| row.blocked()) {
            return Err(format!(
                "'{name}' changed since its findings were read — nothing was changed; review the new findings and accept again"
            ));
        }
    }
    apply::execute(env, &report.plan, None).map_err(|e| e.to_string())?;
    Ok(view(env, scope))
}

#[tauri::command(async)]
#[specta::specta]
pub fn apply_plan(
    scope: Scope,
    remove_orphans: bool,
    allow_unsafe: Vec<String>,
) -> Result<AuditView, String> {
    apply_scope(&env()?, &scope, remove_orphans, allow_unsafe)
}

#[tauri::command(async)]
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
    Ok(view(&env, &scope))
}

#[tauri::command(async)]
#[specta::specta]
pub fn toggle_item(
    scope: Scope,
    kind: ItemKind,
    name: String,
    enabled: bool,
) -> Result<AuditView, String> {
    let env = env()?;
    let report = ops::toggle(
        &env,
        &scope,
        std::slice::from_ref(&name),
        Some(kind),
        enabled,
    )
    .map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}

#[tauri::command(async)]
#[specta::specta]
pub fn remove_item(scope: Scope, kind: ItemKind, name: String) -> Result<AuditView, String> {
    let env = env()?;
    // Removing one item never takes its unneeded leftovers with it here:
    // the page has nowhere to preview that yet, and a sweep the user did
    // not see is exactly the surprise the preview step exists to stop.
    let report = ops::remove(&env, &scope, std::slice::from_ref(&name), Some(kind), false)
        .map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}

/// One recorded acceptance, as the Settings page lists it. `key` is the
/// manifest's own spelling and is what revoke takes back, so even an entry
/// a hand edit mangled can still be withdrawn; the typed fields are parsed
/// from it for display and are absent where the key does not parse.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedOverride {
    pub scope: Scope,
    pub key: String,
    pub kind: Option<ItemKind>,
    pub name: String,
    pub harness: Option<HarnessId>,
    pub granted_at: String,
    /// How many findings the acceptance covered.
    pub findings: u32,
}

fn parse_override_key(key: &str) -> (Option<ItemKind>, String, Option<HarnessId>) {
    let Some((kind_str, rest)) = key.split_once(':') else {
        return (None, key.to_owned(), None);
    };
    let Some((name, harness_str)) = rest.rsplit_once(':') else {
        return (None, key.to_owned(), None);
    };
    let kind = ItemKind::ALL.iter().copied().find(|k| k.name() == kind_str);
    let harness = HarnessId::parse(harness_str);
    match (kind, harness) {
        (Some(kind), Some(harness)) => (Some(kind), name.to_owned(), Some(harness)),
        _ => (None, key.to_owned(), None),
    }
}

#[tauri::command(async)]
#[specta::specta]
pub fn list_safety_overrides() -> Result<Vec<AcceptedOverride>, String> {
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
    let mut accepted = Vec::new();
    for scope in scopes {
        let path = manifest::manifest_path(&env, &scope);
        // A scope whose manifest is unreadable is reported on the audit
        // pages; the acceptances list simply has nothing to say for it.
        let Ok(manifest::ManifestFile::Current(m)) = manifest::load(&path) else {
            continue;
        };
        for (key, recorded) in &m.safety_overrides {
            let (kind, name, harness) = parse_override_key(key);
            accepted.push(AcceptedOverride {
                scope: scope.clone(),
                key: key.clone(),
                kind,
                name,
                harness,
                granted_at: recorded.granted_at.clone(),
                findings: u32::try_from(recorded.findings.len()).unwrap_or(u32::MAX),
            });
        }
    }
    Ok(accepted)
}

#[tauri::command(async)]
#[specta::specta]
pub fn revoke_safety_override(scope: Scope, key: String) -> Result<AuditView, String> {
    let env = env()?;
    let plan = ops::revoke_override(&env, &scope, &key).map_err(|e| e.to_string())?;
    apply::execute(&env, &plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}
