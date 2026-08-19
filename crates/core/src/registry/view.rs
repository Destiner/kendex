//! What the Community tab shows: the directory joined with what this
//! machine already subscribes to, and when the list was really fetched —
//! a row the person already has says "Subscribed", never a second
//! Subscribe button.

use crate::clock;
use crate::env::Env;
use crate::error::Result;
use crate::model::Scope;
use crate::registry::index::{DirectoryBundle, DirectoryPackage};
use crate::registry::{Fetch, cache};
use crate::repo_move;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryView {
    pub rows: Vec<DirectoryRow>,
    /// When the served list was actually fetched (ISO-8601) — the "as of"
    /// line when `stale` is true, the "updated" line otherwise.
    pub fetched_at: String,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRow {
    pub repo: String,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub featured: bool,
    pub package_count: u32,
    pub bundle_count: u32,
    pub subscribed: bool,
    pub packages: Vec<DirectoryPackage>,
    pub bundles: Vec<DirectoryBundle>,
}

pub fn directory(env: &Env, fetch: &dyn Fetch, force_refresh: bool) -> Result<DirectoryView> {
    let loaded = cache::load(env, fetch, force_refresh)?;
    let subscribed = subscribed_repos(env);
    let rows = loaded
        .index
        .marketplaces
        .into_iter()
        .map(|market| {
            let name = market.name.clone().unwrap_or_else(|| leaf_of(&market.repo));
            let is_subscribed =
                repo_move::owner_repo(&market.repo).is_some_and(|key| subscribed.contains(&key));
            DirectoryRow {
                repo: market.repo,
                name,
                description: market.description,
                tags: market.tags,
                featured: market.featured,
                package_count: market.package_count,
                bundle_count: market.bundle_count,
                subscribed: is_subscribed,
                packages: market.packages,
                bundles: market.bundles,
            }
        })
        .collect();
    Ok(DirectoryView {
        rows,
        fetched_at: clock::iso_from_unix(loaded.fetched_at),
        stale: loaded.stale,
    })
}

/// Every repository any scope subscribes to, spelled the one canonical
/// way. A scope that cannot be read contributes nothing rather than
/// blocking the tab.
fn subscribed_repos(env: &Env) -> BTreeSet<String> {
    let mut repos = BTreeSet::new();
    let mut scopes = vec![Scope::Global];
    if let Ok(settings) = crate::settings::load(env) {
        scopes.extend(
            settings
                .projects
                .into_iter()
                .map(|root| Scope::Project { root }),
        );
    }
    for scope in scopes {
        let Ok(rows) = crate::source_ops::list_subscriptions(env, &scope) else {
            continue;
        };
        for row in rows {
            if let Some(key) = row.repo.as_deref().and_then(repo_move::owner_repo) {
                repos.insert(key);
            }
        }
    }
    repos
}

fn leaf_of(repo: &str) -> String {
    repo.rsplit('/').next().unwrap_or(repo).to_string()
}
