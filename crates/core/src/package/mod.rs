//! One installed package, seen through its versions: which revision it is
//! held at, what its source's history offers, and what has changed. The
//! manifest holds the choice (`ItemDecl.rev`), the mirror holds the history,
//! and everything here is a projection over the two.

use crate::engine::EngineReport;
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::model::{ItemKind, Scope};
use crate::source_read::SealedSource;

/// Hold an item at a version, or let it follow its source again.
///
/// The selector may be anything the repository can name — a tag, a branch,
/// a commit — but what the manifest records is always the full commit id it
/// resolves to right now: a name someone can move upstream must never be
/// able to move an item the user chose to hold. Everything is checked
/// before the manifest is touched (invariant 11): the selector must resolve,
/// and the item must actually exist at that commit.
pub fn set_rev(
    env: &Env,
    scope: &Scope,
    kind: ItemKind,
    name: &str,
    rev: Option<&str>,
) -> Result<EngineReport> {
    let mut manifest = crate::engine::ops::manifest_for_mutation(env, scope)?;
    let Some(decl) = manifest.declared(kind).get(name).cloned() else {
        return Err(CoreError::NotDeclared {
            kind,
            name: name.to_owned(),
        });
    };
    let normalized = match rev {
        None => None,
        Some(selector) => {
            let source_decl = manifest.sources.get(&decl.source);
            let Some(repo) = source_decl.and_then(|s| s.repo.clone()) else {
                return Err(CoreError::ItemRevUnsupported {
                    source_name: decl.source.clone(),
                });
            };
            let resolution = resolve_selector(env, &repo, selector)?;
            // A commit the repository holds is not yet a version of this
            // item — the item has to exist in that tree.
            let sealed = SealedSource::open(&resolution.root)?;
            let config = crate::source::source_config(&sealed)?;
            if crate::source::find_item(&sealed, &config, kind, name).is_none() {
                return Err(CoreError::ItemMissingAtRev {
                    name: name.to_owned(),
                    repo,
                    commit: resolution.commit,
                });
            }
            Some(resolution.commit)
        }
    };
    let Some(entry) = manifest.declared_mut(kind).get_mut(name) else {
        return Err(CoreError::NotDeclared {
            kind,
            name: name.to_owned(),
        });
    };
    entry.rev = normalized;
    crate::source_ops::persist_and_plan(env, scope, manifest)
}

/// The cache answers first — a version the mirror already holds needs no
/// network — and the network fills in what it cannot.
fn resolve_selector(env: &Env, repo: &str, selector: &str) -> Result<crate::remote::Resolution> {
    if let Some(resolution) = crate::remote::cached(env, repo, Some(selector))? {
        return Ok(resolution);
    }
    crate::remote::sync(env, repo, Some(selector))
}
