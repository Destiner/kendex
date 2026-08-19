use clap::Subcommand;
use kendex_core::env::Env;
use kendex_core::source_ops;

use super::{CliResult, out, resolve_scopes, say};
use crate::scope::ScopeFilter;

#[derive(Subcommand)]
pub enum MarketplaceCommand {
    /// Subscriptions per scope, with package counts once fetched
    List {
        /// Machine-readable rows (schema 1)
        #[arg(long)]
        json: bool,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
    },
    /// Subscribe to a marketplace: owner/repo[@rev], a git URL, a GitHub
    /// tree URL, a skills.sh package URL, or a local folder
    Subscribe {
        reference: String,
        /// Name for the subscription (default: the last path segment)
        #[arg(long)]
        name: Option<String>,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global (default project)
        #[arg(long)]
        scope: Option<String>,
    },
}

pub fn run(env: &Env, command: MarketplaceCommand) -> CliResult {
    match command {
        MarketplaceCommand::List {
            json,
            global,
            scope,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            let mut rows = Vec::new();
            for scope in resolve_scopes(env, filter)? {
                rows.extend(source_ops::list_subscriptions(env, &scope)?);
            }
            if json {
                out(&serde_json::to_string_pretty(&serde_json::json!({
                    "schema": 1,
                    "subscriptions": rows,
                }))?);
                return Ok(());
            }
            for row in rows {
                let what = row.repo.or(row.path).unwrap_or_default();
                let rev = row.rev.map(|rev| format!(" @ {rev}")).unwrap_or_default();
                let counted = match row.counts {
                    Some(counts) => {
                        let total: usize = counts.values().sum();
                        format!("{total} package(s)")
                    }
                    None => "not fetched yet".to_owned(),
                };
                let state = if row.enabled { "" } else { "  (disabled)" };
                out(&format!(
                    "{}  {}  {what}{rev}  [{counted}]{state}",
                    row.scope.label(),
                    row.name,
                ));
            }
        }
        MarketplaceCommand::Subscribe {
            reference,
            name,
            global,
            scope,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::Project)?;
            let scope = resolve_scopes(env, filter)?.remove(0);
            let subscribed = source_ops::subscribe(env, &scope, &reference, name.as_deref())?;
            for note in &subscribed.report.notes {
                say(note);
            }
            kendex_core::apply::execute(env, &subscribed.report.plan, None)?;
            // Subscribing fetches so counts can land; a failure costs the
            // counts, never the subscription.
            if let Ok(Some(manifest)) = kendex_core::manifest::load_for_mutation(
                &kendex_core::manifest::manifest_path(env, &scope),
            ) && let Some(decl) = manifest.sources.get(&subscribed.name)
                && let Some(repo) = decl.repo.clone()
                && let Err(error) = kendex_core::remote::sync(env, &repo, decl.rev.as_deref())
            {
                say(&format!("warning: not fetched yet ({error})"));
            }
            say(&format!(
                "{}: subscribed to '{}' ({})",
                scope.label(),
                subscribed.name,
                subscribed.reference
            ));
            if let Some(lead) = subscribed.lead {
                say(&format!("package: {lead}"));
            }
        }
    }
    Ok(())
}
