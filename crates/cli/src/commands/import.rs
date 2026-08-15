use std::fs;
use std::path::PathBuf;

use vstack_core::env::Env;
use vstack_core::import_v1::convert;
use vstack_core::model::Scope;
use vstack_core::{lock, manifest};

use super::{CliResult, out, resolve_scopes, say};
use crate::scope::ScopeFilter;

fn v1_lock_path(env: &Env, scope: &Scope) -> PathBuf {
    match scope {
        Scope::Global => env.home.join(".config/vstack/.vstack-lock.json"),
        Scope::Project { root } => root.join(".vstack-lock.json"),
    }
}

fn backup(env: &Env, path: &PathBuf) -> CliResult {
    if !path.exists() {
        return Ok(());
    }
    let trash = env.trash_dir();
    fs::create_dir_all(&trash)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "v1-file".to_owned());
    let stamp = vstack_core::clock::timestamp().replace(':', "-");
    fs::copy(path, trash.join(format!("{stamp}-v1-{name}")))?;
    Ok(())
}

/// One-shot v1 → v2 migration for a scope. Idempotent: an already-migrated
/// scope is skipped; a half-migrated one (v2 manifest, v1 lock) converts
/// the remainder. Originals are copied to the trash first.
pub fn run(env: &Env, filter: ScopeFilter) -> CliResult {
    let mut migrated = 0usize;
    for scope in resolve_scopes(env, filter)? {
        let manifest_path = manifest::manifest_path(env, &scope);
        let v1_manifest = match manifest::load(&manifest_path)? {
            manifest::ManifestFile::Legacy { raw } => Some(raw),
            manifest::ManifestFile::Current(_) => None,
            manifest::ManifestFile::Absent => None,
        };
        let already_current = matches!(
            manifest::load(&manifest_path)?,
            manifest::ManifestFile::Current(_)
        );

        let v1_lock_file = v1_lock_path(env, &scope);
        let v1_lock = fs::read_to_string(&v1_lock_file)
            .ok()
            .filter(|text| lock::is_v1_text(text));

        if v1_manifest.is_none() && v1_lock.is_none() {
            if already_current {
                say(&format!("{}: already migrated", scope.label()));
            }
            continue;
        }

        let outcome = convert(v1_manifest.as_deref(), v1_lock.as_deref())
            .map_err(|e| format!("{}: {e}", scope.label()))?;
        for note in &outcome.notes {
            say(&format!("{}: {note}", scope.label()));
        }

        backup(env, &manifest_path)?;
        backup(env, &v1_lock_file)?;

        if already_current {
            // Keep the existing v2 manifest; only the v1 lock converts.
            lock::save(&lock::lock_path(env, &scope), &outcome.lock)?;
        } else {
            manifest::save(&manifest_path, &outcome.manifest)?;
            lock::save(&lock::lock_path(env, &scope), &outcome.lock)?;
        }
        // The v1 global lock lives in v1's own dir and would re-trigger
        // import forever; project scope reuses the same path as v2, where
        // the save above already replaced it.
        if matches!(scope, Scope::Global) && v1_lock.is_some() {
            fs::remove_file(&v1_lock_file)?;
        }
        migrated += 1;
        out(&format!(
            "{}: migrated ({} lock entries)",
            scope.label(),
            outcome.lock.entries.len()
        ));
    }
    if migrated == 0 {
        say("nothing to migrate");
    } else {
        say("run `vstack refresh` to regenerate everything from sources");
    }
    Ok(())
}
