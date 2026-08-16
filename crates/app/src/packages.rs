//! The package page's and Updates page's commands: versions, diffs, files,
//! provenance, holds, forks, and the update check — thin shells over core,
//! like every other command here.

use vstack_core::apply;
use vstack_core::engine;
use vstack_core::env::Env;
use vstack_core::model::{HarnessId, ItemKind, Scope};
use vstack_core::package::{self, detail, diff, updates};
use vstack_core::{manifest, remote};

use crate::audit::{AuditView, view};

fn env() -> Result<Env, String> {
    Env::detect().map_err(|e| e.to_string())
}

fn all_scopes(env: &Env) -> Result<Vec<Scope>, String> {
    let settings = vstack_core::settings::load(env).map_err(|e| e.to_string())?;
    let mut scopes = vec![Scope::Global];
    scopes.extend(
        settings
            .projects
            .into_iter()
            .map(|root| Scope::Project { root }),
    );
    Ok(scopes)
}

#[tauri::command(async)]
#[specta::specta]
pub fn package_versions(
    scope: Scope,
    kind: ItemKind,
    name: String,
) -> Result<Vec<package::VersionRow>, String> {
    let env = env()?;
    package::versions(&env, &scope, kind, &name).map_err(|e| e.to_string())
}

/// Every scope's update standing in one query — the sidebar badge, the
/// Updates page, and the Library's fork/edited flags all read this.
#[tauri::command(async)]
#[specta::specta]
pub fn updates_overview() -> Result<Vec<updates::UpdateRow>, String> {
    let env = env()?;
    let mut rows = Vec::new();
    for scope in all_scopes(&env)? {
        rows.extend(updates::updates(&env, &scope).map_err(|e| e.to_string())?);
    }
    Ok(rows)
}

/// Fetch every source's mirror — pinned ones included, that is the point —
/// then answer with the fresh standing. Fetch problems degrade to
/// warnings; a check for updates is never worth an error dialog.
#[tauri::command(async)]
#[specta::specta]
pub fn updates_refresh() -> Result<Vec<updates::UpdateRow>, String> {
    let env = env()?;
    for scope in all_scopes(&env)? {
        let path = manifest::manifest_path(&env, &scope);
        if let Ok(Some(loaded)) = manifest::load_for_mutation(&path) {
            let _warnings = remote::fetch_all(&env, &loaded);
        }
    }
    updates_overview()
}

#[tauri::command(async)]
#[specta::specta]
pub fn update_set_ignored(
    scope: Scope,
    kind: ItemKind,
    name: String,
    repo: String,
    ignored: bool,
) -> Result<Vec<updates::UpdateRow>, String> {
    let env = env()?;
    updates::set_ignored(&env, &scope, kind, &name, &repo, ignored).map_err(|e| e.to_string())?;
    updates_overview()
}

/// Hold a package at a version (or let it follow again) and apply the
/// change. The plan is whole-scope, like every apply: packages that follow
/// their source come current in the same pass, which is what following
/// means.
#[tauri::command(async)]
#[specta::specta]
pub fn package_set_rev(
    scope: Scope,
    kind: ItemKind,
    name: String,
    rev: Option<String>,
) -> Result<AuditView, String> {
    let env = env()?;
    let report =
        package::set_rev(&env, &scope, kind, &name, rev.as_deref()).map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}

#[tauri::command(async)]
#[specta::specta]
pub fn package_diff(
    scope: Scope,
    kind: ItemKind,
    name: String,
    from: diff::VersionSel,
    to: diff::VersionSel,
    harness: Option<HarnessId>,
) -> Result<diff::PackageDiff, String> {
    let env = env()?;
    diff::package_diff(&env, &scope, kind, &name, &from, &to, harness).map_err(|e| e.to_string())
}

/// Keep an edited install as a local fork, then render it in place.
#[tauri::command(async)]
#[specta::specta]
pub fn package_fork(
    scope: Scope,
    kind: ItemKind,
    name: String,
    harness: HarnessId,
) -> Result<AuditView, String> {
    let env = env()?;
    let plan = engine::fork::fork(&env, &scope, kind, &name, harness).map_err(|e| e.to_string())?;
    apply::execute(&env, &plan, None).map_err(|e| e.to_string())?;
    let report = engine::audit(&env, &scope).map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}

#[tauri::command(async)]
#[specta::specta]
pub fn fork_rename(
    scope: Scope,
    kind: ItemKind,
    old_name: String,
    new_name: String,
) -> Result<AuditView, String> {
    let env = env()?;
    let plan = engine::fork::rename_fork(&env, &scope, kind, &old_name, &new_name)
        .map_err(|e| e.to_string())?;
    apply::execute(&env, &plan, None).map_err(|e| e.to_string())?;
    // The old name's artifacts come off disk with the rename — the user
    // asked for this by name, which is what an explicit removal is.
    let report = vstack_core::engine::plan_scope(
        &env,
        &scope,
        &manifest::load_for_mutation(&manifest::manifest_path(&env, &scope))
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no manifest".to_owned())?,
        &vstack_core::lock::load(&vstack_core::lock::lock_path(&env, &scope))
            .map_err(|e| e.to_string())?,
        &engine::PlanOptions {
            remove_orphans: true,
            removal_filter: Some(vec![old_name]),
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}

/// Apply a scope with one package's edits discarded — the door back to
/// "the catalog's version wins", scoped to the package the user named so
/// a neighbour's edits are never taken along.
#[tauri::command(async)]
#[specta::specta]
pub fn apply_discard_edits(scope: Scope, name: String) -> Result<AuditView, String> {
    let env = env()?;
    let manifest = manifest::load_for_mutation(&manifest::manifest_path(&env, &scope))
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no manifest".to_owned())?;
    let lock = vstack_core::lock::load(&vstack_core::lock::lock_path(&env, &scope))
        .map_err(|e| e.to_string())?;
    let report = vstack_core::engine::plan_scope(
        &env,
        &scope,
        &manifest,
        &lock,
        &engine::PlanOptions {
            overwrite_edited_names: Some(vec![name]),
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    apply::execute(&env, &report.plan, None).map_err(|e| e.to_string())?;
    Ok(view(&env, &scope))
}

#[tauri::command(async)]
#[specta::specta]
pub fn package_files(
    scope: Scope,
    kind: ItemKind,
    name: String,
) -> Result<Vec<detail::PackageFile>, String> {
    let env = env()?;
    detail::package_files(&env, &scope, kind, &name).map_err(|e| e.to_string())
}

#[tauri::command(async)]
#[specta::specta]
pub fn package_file(
    scope: Scope,
    kind: ItemKind,
    name: String,
    path: String,
) -> Result<engine::ItemSource, String> {
    let env = env()?;
    detail::package_file(&env, &scope, kind, &name, &path).map_err(|e| e.to_string())
}

#[tauri::command(async)]
#[specta::specta]
pub fn package_readme(
    scope: Scope,
    kind: ItemKind,
    name: String,
) -> Result<Option<engine::ItemSource>, String> {
    let env = env()?;
    detail::package_readme(&env, &scope, kind, &name).map_err(|e| e.to_string())
}

#[tauri::command(async)]
#[specta::specta]
pub fn package_meta(
    scope: Scope,
    kind: ItemKind,
    name: String,
) -> Result<detail::PackageMeta, String> {
    let env = env()?;
    detail::package_meta(&env, &scope, kind, &name).map_err(|e| e.to_string())
}
