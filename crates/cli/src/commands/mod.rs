pub mod add;
pub mod adopt;
pub mod apply_cmd;
pub mod check;
pub mod check_catalog;
pub mod engine_common;
pub mod import;
pub mod init;
pub mod list;
pub mod pin;
pub mod project;
pub mod refresh;
pub mod remove;
pub mod report;
pub mod source_cmd;
pub mod update;
pub mod update_pi;
pub mod updates_cmd;
pub mod verify;
pub mod versions;

use std::io::Write;
use std::path::PathBuf;

use vstack_core::discover;
use vstack_core::env::Env;
use vstack_core::model::Scope;

use crate::scope::ScopeFilter;

pub type CliResult = Result<(), Box<dyn std::error::Error>>;

/// v1 prints human tables to stderr; stdout stays clean for composition.
pub fn say(line: &str) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

pub fn out(line: &str) {
    let _ = writeln!(std::io::stdout(), "{line}");
}

/// The scopes a filter selects on this machine: the current project (walked
/// up from CWD, v1 rules) and/or global.
pub fn resolve_scopes(env: &Env, filter: ScopeFilter) -> Result<Vec<Scope>, String> {
    let current = current_project(env);
    match filter {
        ScopeFilter::Global => Ok(vec![Scope::Global]),
        ScopeFilter::Project => match current {
            Some(root) => Ok(vec![Scope::Project { root }]),
            None => Err("not inside a project (no harness marker found walking up)".to_owned()),
        },
        ScopeFilter::All => {
            let mut scopes: Vec<Scope> = current
                .map(|root| Scope::Project { root })
                .into_iter()
                .collect();
            scopes.push(Scope::Global);
            Ok(scopes)
        }
    }
}

fn current_project(env: &Env) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    discover::project_root_from(&cwd, &env.home)
}
