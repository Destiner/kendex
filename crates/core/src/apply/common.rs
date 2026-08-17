//! Common-state applies: mutations that live in a repository's git common
//! dir (the shared hooks directory, the repo's config), locked and
//! journaled per common dir rather than per worktree scope — two linked
//! worktrees share the state, so they must share the lock and the journal.

use std::fs;
use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::error::{CoreError, Result};

use super::{ApplyOutcome, Plan, ScopeGuard, created_dir_roots, journal, lock_scope};

/// Filesystem-safe key naming a repository's common-dir lock and journal.
/// Hook state is repository-common state: two linked worktrees share one
/// hooks directory, and a lock keyed per worktree would let them mutate it
/// under different locks.
pub fn common_key(common_dir: &Path) -> String {
    let canonical = common_dir
        .canonicalize()
        .unwrap_or_else(|_| common_dir.to_path_buf());
    let text = canonical.display().to_string();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("git-common-{hash:016x}")
}

/// Execute a plan whose mutations live in repository-common state (the
/// shared hooks directory, the repo's git config). Lock order is fixed:
/// the scope lock first, then the common-dir lock — and every common-lock
/// holder recovers the common journal before mutating, so one worktree's
/// crash is rolled back by whichever worktree gets there next, never
/// under a different lock.
pub fn execute_common(env: &Env, plan: &Plan, common_dir: &Path) -> Result<ApplyOutcome> {
    let _scope_guard = lock_scope(env, &plan.scope)?;
    let key = common_key(common_dir);
    let _common_guard = lock_common(env, &key)?;
    let journal_dir = journal::journal_dir_for(&env.journal_dir(), &key);
    let recovered_first = if journal::pending(&journal_dir) {
        journal::rollback(&journal_dir)?;
        true
    } else {
        journal::clear(&journal_dir)?;
        false
    };
    let mut touched: Vec<PathBuf> = plan.ops.iter().flat_map(|p| p.op.touched()).collect();
    touched.extend(created_dir_roots(&touched));
    journal::write(&journal_dir, &touched)?;
    for planned in &plan.ops {
        if let Err(error) = planned.op.run(env) {
            journal::rollback(&journal_dir)?;
            return Err(CoreError::RolledBack {
                reason: format!("'{}' failed: {error}", planned.description),
            });
        }
    }
    journal::clear(&journal_dir)?;
    Ok(ApplyOutcome {
        applied: plan.ops.len(),
        recovered_first,
    })
}

/// Recover an interrupted common-state apply, under the common lock —
/// what every hook mutation runs before planning against current state.
pub fn recover_common(env: &Env, common_dir: &Path) -> Result<bool> {
    let key = common_key(common_dir);
    let _guard = lock_common(env, &key)?;
    let dir = journal::journal_dir_for(&env.journal_dir(), &key);
    if journal::pending(&dir) {
        journal::rollback(&dir)?;
        return Ok(true);
    }
    journal::clear(&dir)?;
    Ok(false)
}

fn lock_common(env: &Env, key: &str) -> Result<ScopeGuard> {
    let dir = env.scope_locks_dir();
    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    let path = dir.join(format!("{key}.lock"));
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| CoreError::io(&path, e))?;
    let mut lock = fd_lock::RwLock::new(file);
    let acquired = match lock.try_write() {
        Ok(guard) => {
            std::mem::forget(guard);
            true
        }
        Err(_) => false,
    };
    match acquired {
        true => Ok(ScopeGuard {
            _file: lock.into_inner(),
        }),
        false => Err(CoreError::ScopeBusy { lock: path }),
    }
}
