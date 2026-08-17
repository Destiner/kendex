use std::fs;
use std::path::PathBuf;

use vstack_core::apply::{Op, Plan, PlannedOp, Pre};
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

/// The v1 lock this scope carries, if any. A file that exists but is
/// neither a v1 lock nor a current one is a refusal — treating it as
/// absent would bury a damaged record under a fresh empty lock (the
/// #1307 class).
fn v1_lock(env: &Env, scope: &Scope) -> Result<Option<String>, String> {
    let path = v1_lock_path(env, scope);
    // Project scope shares the path with v2; lock::load_file has already
    // classified what sits there, so only the global scope's separate v1
    // path needs reading here.
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(None);
    };
    if lock::is_v1_text(&text) {
        return Ok(Some(text));
    }
    if matches!(lock::load_file(&path), Ok(lock::LockFile::Current(_))) {
        return Ok(None);
    }
    Err(format!(
        "{} exists but is neither a v1 lock nor a current one — inspect it; refusing to treat a damaged record as absent",
        path.display()
    ))
}

/// One-shot v1 → v2 migration for a scope: one journaled plan whose
/// preconditions bind to the exact files read, refusing rather than
/// overwriting anything live. Originals are copied to the trash first.
pub fn run(env: &Env, filter: ScopeFilter) -> CliResult {
    let mut migrated = 0usize;
    for scope in resolve_scopes(env, filter)? {
        let manifest_path = manifest::manifest_path(env, &scope);
        let manifest_file = manifest::load(&manifest_path)?;
        let v1_manifest = match &manifest_file {
            manifest::ManifestFile::Legacy { raw } => Some(raw.clone()),
            _ => None,
        };
        let already_current = matches!(manifest_file, manifest::ManifestFile::Current(_));

        let v1_lock_file = v1_lock_path(env, &scope);
        // A corrupt v2-side lock also surfaces here (shared path in a
        // project) and refuses the import instead of burying the damage.
        let v2_lock_path = lock::lock_path(env, &scope);
        let v2_lock_state = lock::load_file(&v2_lock_path)?;
        let v1_lock = match &v2_lock_state {
            lock::LockFile::Legacy { raw } if v2_lock_path == v1_lock_file => Some(raw.clone()),
            _ => v1_lock(env, &scope).map_err(|e| format!("{}: {e}", scope.label()))?,
        };

        if v1_manifest.is_none() && v1_lock.is_none() {
            if already_current {
                say(&format!("{}: already migrated", scope.label()));
            }
            continue;
        }

        // The destination must be empty: a live v2 install record is
        // current provenance, and re-importing a stale v1 lock over it
        // would replace truth with history (the #1307 class).
        if let lock::LockFile::Current(current) = &v2_lock_state
            && !current.entries.is_empty()
        {
            return Err(format!(
                "{}: this scope already has a live v2 install record ({} entries) — refusing to import over it; remove the stale v1 leftovers instead ({})",
                scope.label(),
                current.entries.len(),
                v1_lock_file.display()
            )
            .into());
        }

        let outcome = convert(v1_manifest.as_deref(), v1_lock.as_deref())
            .map_err(|e| format!("{}: {e}", scope.label()))?;
        for note in &outcome.notes {
            say(&format!("{}: {note}", scope.label()));
        }

        backup(env, &manifest_path)?;
        backup(env, &v1_lock_file)?;

        // One journaled plan: preconditions bind to the bytes that were
        // read, the journal rolls a failure back whole, and the scope lock
        // keeps a concurrent writer out.
        let mut ops = Vec::new();
        if !already_current {
            ops.push(PlannedOp {
                description: "write the migrated vstack.toml".into(),
                op: Op::WriteManifest {
                    pre: Pre::observed(&manifest_path)?,
                    path: manifest_path.clone(),
                    manifest: Box::new(outcome.manifest.clone()),
                },
            });
        }
        ops.push(PlannedOp {
            description: "write the migrated install record".into(),
            op: Op::WriteLock {
                pre: Pre::observed(&v2_lock_path)?,
                path: v2_lock_path.clone(),
                lock: Box::new(outcome.lock.clone()),
            },
        });
        // The v1 global lock lives in v1's own dir and would re-trigger
        // import forever; a project's shares the v2 path, replaced above.
        if v1_lock_file != v2_lock_path && v1_lock.is_some() {
            ops.push(PlannedOp {
                description: "retire the v1 lock".into(),
                op: Op::Trash {
                    pre: Pre::observed(&v1_lock_file)?,
                    path: v1_lock_file.clone(),
                },
            });
        }
        vstack_core::apply::execute(
            env,
            &Plan {
                scope: scope.clone(),
                ops,
            },
            None,
        )?;
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
