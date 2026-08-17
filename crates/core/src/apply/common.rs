//! Common-state applies: mutations that live in a repository's git common
//! dir (the shared hooks directory, the repo's config), locked and
//! journaled per common dir rather than per worktree scope — two linked
//! worktrees share the state, so they must share the lock and the journal.

use std::path::Path;

use crate::env::Env;
use crate::error::Result;
use crate::model::Scope;

use super::{ApplyOutcome, PlannedOp, lock_key, lock_scope, recover_key, run_journaled};

const KEY_PREFIX: &str = "git-common-";

/// Filesystem-safe key naming a repository's common-dir lock and journal.
/// Hook state is repository-common state: two linked worktrees share one
/// hooks directory, and a lock keyed per worktree would let them mutate it
/// under different locks.
pub fn common_key(common_dir: &Path) -> String {
    let canonical = common_dir
        .canonicalize()
        .unwrap_or_else(|_| common_dir.to_path_buf());
    let text = canonical.display().to_string();
    format!("{KEY_PREFIX}{}", crate::hash::fnv1a_hex(text.as_bytes()))
}

/// Execute a mutation of repository-common state. Lock order is fixed —
/// the scope lock first, then the common-dir lock — and the plan is built
/// by `build` only once both are held and the common journal is
/// recovered: every refusal and precondition it observes is observed
/// under the lock, so no other writer can change the directory's shape
/// between the check and the write. `build` returns the ops plus whatever
/// the caller wants back beside the outcome.
pub fn execute_common<T>(
    env: &Env,
    scope: &Scope,
    common_dir: &Path,
    build: impl FnOnce() -> Result<(Vec<PlannedOp>, T)>,
) -> Result<(ApplyOutcome, T)> {
    let _scope_guard = lock_scope(env, scope)?;
    let key = common_key(common_dir);
    let _common_guard = lock_key(env, &key)?;
    let recovered_first = recover_key(env, &key)?;
    let (ops, extra) = build()?;
    let applied = run_journaled(env, &ops, &key, None)?;
    Ok((
        ApplyOutcome {
            applied,
            recovered_first,
        },
        extra,
    ))
}

/// Recover every interrupted common-state apply this machine's journal
/// dir records — the launch pass, which otherwise only knows scopes. Each
/// key reports whether it recovered, or why it could not (a busy key has
/// a live writer that recovers it itself).
pub fn recover_common_journals(env: &Env) -> Vec<(String, Result<bool>)> {
    let base = env.journal_dir();
    let Ok(entries) = std::fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(KEY_PREFIX))
        .collect();
    keys.sort();
    keys.into_iter()
        .map(|key| {
            let result = lock_key(env, &key).and_then(|_guard| recover_key(env, &key));
            (key, result)
        })
        .collect()
}
