use kendex_core::env::Env;
use kendex_core::import_v1::migrate;

use super::{CliResult, out, resolve_scopes, say};
use crate::scope::ScopeFilter;

/// One-shot v1 → v2 migration, one journaled plan per scope. Thin shell:
/// the classification, refusals, and preconditions all live in core
/// (`import_v1::migrate`). A refusal in one scope is reported and the
/// remaining scopes still migrate — the run fails at the end, after doing
/// everything it honestly could.
pub fn run(env: &Env, filter: ScopeFilter) -> CliResult {
    let mut migrated = 0usize;
    let mut refusals = Vec::new();
    for scope in resolve_scopes(env, filter)? {
        match migrate::migrate_scope(env, &scope) {
            Ok(outcome) => {
                for note in &outcome.notes {
                    say(&format!("{}: {note}", scope.label()));
                }
                if let Some(entries) = outcome.migrated {
                    migrated += 1;
                    out(&format!(
                        "{}: migrated ({entries} lock entries)",
                        scope.label()
                    ));
                }
            }
            Err(error) => {
                say(&format!("refused: {error}"));
                refusals.push(error.to_string());
            }
        }
    }
    if migrated == 0 && refusals.is_empty() {
        say("nothing to migrate");
    } else if migrated > 0 {
        say("run `kendex refresh` to regenerate everything from sources");
    }
    if !refusals.is_empty() {
        return Err(format!("{} scope(s) refused to migrate — see above", refusals.len()).into());
    }
    Ok(())
}
