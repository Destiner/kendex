use clap::Subcommand;
use vstack_core::env::Env;
use vstack_core::{remote, source_ops};

use super::{CliResult, out, resolve_scopes, say};
use crate::scope::ScopeFilter;

#[derive(Subcommand)]
pub enum SourceCommand {
    /// List declared sources for the scope
    List,
    /// Declare a source: `owner/repo` or a local path
    Add { name: String, reference: String },
    /// Remove a source (blocked while items still reference it)
    Remove { name: String },
    /// Re-enable a source and restore its installations
    Enable { name: String },
    /// Disable a source; its installations deactivate but stay declared
    Disable { name: String },
    /// Re-resolve remote source caches
    Refresh,
}

pub fn run(env: &Env, command: SourceCommand, filter: ScopeFilter) -> CliResult {
    for scope in resolve_scopes(env, filter)? {
        match &command {
            SourceCommand::List => {
                for row in source_ops::list_sources(env, &scope)? {
                    let state = if row.enabled { "" } else { "  (disabled)" };
                    let head = row
                        .head
                        .as_deref()
                        .map(|h| format!("  @{h}"))
                        .unwrap_or_default();
                    out(&format!(
                        "{}  {}  {}{head}{state}  [{} item(s)]",
                        scope.label(),
                        row.name,
                        row.reference,
                        row.declared_items.len()
                    ));
                }
            }
            SourceCommand::Add { name, reference } => {
                let report = source_ops::add_source(env, &scope, name, reference)?;
                vstack_core::apply::execute(env, &report.plan, None)?;
                say(&format!("{}: declared source '{name}'", scope.label()));
            }
            SourceCommand::Remove { name } => {
                let report = source_ops::remove_source(env, &scope, name)?;
                vstack_core::apply::execute(env, &report.plan, None)?;
                say(&format!("{}: removed source '{name}'", scope.label()));
            }
            SourceCommand::Enable { name } | SourceCommand::Disable { name } => {
                let enabled = matches!(command, SourceCommand::Enable { .. });
                let report = source_ops::toggle_source(env, &scope, name, enabled)?;
                vstack_core::apply::execute(env, &report.plan, None)?;
                say(&format!(
                    "{}: source '{name}' {}",
                    scope.label(),
                    if enabled { "enabled" } else { "disabled" }
                ));
            }
            SourceCommand::Refresh => {
                let Some(manifest) = vstack_core::manifest::load_for_mutation(
                    &vstack_core::manifest::manifest_path(env, &scope),
                )?
                else {
                    continue;
                };
                for warning in remote::sync_sources(env, &manifest)? {
                    say(&format!("warning: {warning}"));
                }
                say(&format!("{}: sources refreshed", scope.label()));
            }
        }
    }
    Ok(())
}
