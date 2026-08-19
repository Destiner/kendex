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
    /// Unsubscribe from a marketplace, removing or keeping its packages
    Unsubscribe {
        name: String,
        /// Uninstall everything installed from it
        #[arg(long, conflicts_with = "keep_packages")]
        remove_packages: bool,
        /// Keep its packages as your own local forks
        #[arg(long)]
        keep_packages: bool,
        /// With --remove-packages: discard hand edits too, instead of refusing
        #[arg(long)]
        discard_edits: bool,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global (default project)
        #[arg(long)]
        scope: Option<String>,
    },
    /// Packages and curated sets a subscription offers
    Browse {
        /// The subscription to browse (default: every subscription in scope)
        marketplace: Option<String>,
        /// Machine-readable rows (schema 1)
        #[arg(long)]
        json: bool,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
        /// The community directory (not available yet — coming with the
        /// kendex.ai platform)
        #[arg(long)]
        community: bool,
    },
    /// Validate a marketplace directory — the alias of
    /// `check --catalog --strict`
    Check {
        /// The marketplace directory (default: the current directory)
        dir: Option<std::path::PathBuf>,
    },
}

type BrowseRow = (
    kendex_core::model::Scope,
    String,
    kendex_core::source::browse::AvailablePackage,
);

fn run_browse(
    env: &Env,
    marketplace: Option<String>,
    json: bool,
    global: bool,
    scope: Option<String>,
    community: bool,
) -> CliResult {
    if community {
        return Err(
            "the community directory is not available yet — it arrives with the kendex.ai platform; browse a subscription by name for now"
                .into(),
        );
    }
    let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
    let mut rows: Vec<BrowseRow> = Vec::new();
    for scope in resolve_scopes(env, filter)? {
        let names: Vec<String> = match &marketplace {
            Some(name) => vec![name.clone()],
            None => source_ops::list_subscriptions(env, &scope)?
                .into_iter()
                .map(|row| row.name)
                .collect(),
        };
        for name in names {
            // A subscription that will not open costs its own rows, not the
            // whole listing — the same tolerance the app's overview shows.
            let Ok(packages) = kendex_core::source::browse::packages(env, &scope, &name) else {
                continue;
            };
            for package in packages {
                rows.push((scope.clone(), name.clone(), package));
            }
        }
    }
    if json {
        let items: Vec<_> = rows
            .iter()
            .map(|(scope, marketplace, package)| {
                serde_json::json!({
                    "scope": scope.label(),
                    "marketplace": marketplace,
                    "package": package,
                })
            })
            .collect();
        out(&serde_json::to_string_pretty(&serde_json::json!({
            "schema": 1,
            "packages": items,
        }))?);
        return Ok(());
    }
    for (scope, marketplace, package) in rows {
        let description = package
            .description
            .map(|d| format!("  — {d}"))
            .unwrap_or_default();
        out(&format!(
            "{}  {marketplace}::{}  ({}) [{}]{description}",
            scope.label(),
            package.name,
            package.kind.name(),
            install_state(&package.state),
        ));
    }
    Ok(())
}

fn run_unsubscribe(
    env: &Env,
    name: &str,
    remove_packages: bool,
    keep_packages: bool,
    discard_edits: bool,
    global: bool,
    scope: Option<String>,
) -> CliResult {
    use kendex_core::engine::detach;
    let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::Project)?;
    let scope = resolve_scopes(env, filter)?.remove(0);

    let manifest = kendex_core::engine::ops::manifest_for_mutation(env, &scope)?;
    let closure = detach::closure(env, &scope, name, &manifest)?;

    // Nothing installed: a plain confirm, whichever flag (or none) was passed.
    if closure.items.is_empty() {
        let report = kendex_core::source_ops::remove_source(env, &scope, name)?;
        kendex_core::apply::execute(env, &report.plan, None)?;
        say(&format!("{}: unsubscribed from '{name}'", scope.label()));
        return Ok(());
    }

    let plan = match (remove_packages, keep_packages) {
        (false, false) => {
            return Err(format!(
                "'{name}' has {} package(s) installed — pass --remove-packages to uninstall them or --keep-packages to keep them as your own",
                closure.items.len()
            )
            .into());
        }
        (_, true) => detach::source(env, &scope, name)?,
        (true, _) => detach::remove(env, &scope, name, discard_edits)?.plan,
    };
    kendex_core::apply::execute(env, &plan, None)?;
    let kept = if keep_packages { "kept" } else { "removed" };
    say(&format!(
        "{}: unsubscribed from '{name}', {} {kept}",
        scope.label(),
        closure.items.len()
    ));
    Ok(())
}

fn install_state(state: &kendex_core::source::browse::InstallState) -> &'static str {
    use kendex_core::source::browse::InstallState;
    match state {
        InstallState::Installed => "installed",
        InstallState::Available => "available",
        InstallState::HeldBackBySafety => "held back by safety",
        InstallState::NotOffered => "no longer offered",
    }
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
        MarketplaceCommand::Unsubscribe {
            name,
            remove_packages,
            keep_packages,
            discard_edits,
            global,
            scope,
        } => run_unsubscribe(
            env,
            &name,
            remove_packages,
            keep_packages,
            discard_edits,
            global,
            scope,
        )?,
        MarketplaceCommand::Browse {
            marketplace,
            json,
            global,
            scope,
            community,
        } => run_browse(env, marketplace, json, global, scope, community)?,
        MarketplaceCommand::Check { dir } => {
            let dir = match dir {
                Some(dir) => dir,
                None => std::env::current_dir()?,
            };
            super::check_catalog::run(&dir, true, false)?;
        }
    }
    Ok(())
}
