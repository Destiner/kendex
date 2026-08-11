use std::process::ExitCode;

use vstack_core::engine::{DriftState, audit};
use vstack_core::env::Env;
use vstack_core::lock::{load as load_lock, lock_path};

use super::{resolve_scopes, say};
use crate::scope::ScopeFilter;

/// Drift check over lock entries; non-zero exit on any failing row — this
/// is the signal consuming repos compose in shell pipelines.
pub fn run(
    env: &Env,
    names: Vec<String>,
    filter: ScopeFilter,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut checked = 0usize;
    let mut failed = 0usize;

    for scope in resolve_scopes(env, filter)? {
        let lock = load_lock(&lock_path(env, &scope))?;
        if lock.entries.is_empty() {
            continue;
        }
        let report = audit(env, &scope)?;
        for entry in lock.entries.values() {
            if !names.is_empty() && !names.contains(&entry.name) {
                continue;
            }
            checked += 1;
            let problem = report.drift.iter().find(|row| {
                row.name == entry.name
                    && row.kind == entry.kind
                    && row.harness == entry.harness
                    && matches!(
                        row.state,
                        DriftState::Missing | DriftState::Stale | DriftState::Conflict
                    )
            });
            let unreachable_source = report.notes.iter().any(|n| {
                n.starts_with(&format!("{}:", entry.name)) && !n.contains("disabled — inactive")
            });
            match problem {
                Some(row) => {
                    failed += 1;
                    say(&format!(
                        "✗ {} {} [{}]: {}",
                        entry.kind.name(),
                        entry.name,
                        entry.harness.name(),
                        row.detail
                    ));
                }
                None if unreachable_source => {
                    failed += 1;
                    say(&format!(
                        "✗ {} {} [{}]: source unavailable",
                        entry.kind.name(),
                        entry.name,
                        entry.harness.name()
                    ));
                }
                None => say(&format!(
                    "✓ {} {} [{}]",
                    entry.kind.name(),
                    entry.name,
                    entry.harness.name()
                )),
            }
        }
    }

    if checked == 0 {
        say("nothing installed");
        return Ok(ExitCode::SUCCESS);
    }
    say(&format!(
        "{checked} checked, {} OK, {failed} failed",
        checked - failed
    ));
    Ok(if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
