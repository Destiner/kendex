//! Writing one item's declaration into the manifest: the invariant-4
//! collision refusal (installed or merely declared), the `--hold` commit, and
//! the source label a collision names.

use super::AddRequest;
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::lock::Lock;
use crate::manifest::{ItemDecl, Manifest, Method};
use crate::model::{ItemKind, Scope};
use crate::source;

/// The commit a `--hold` request freezes its declarations at. Only a
/// remote resolves to one; a hold on anything else is refused before the
/// first declaration is written (invariant 11).
pub(super) fn hold_commit(
    request: &AddRequest,
    source_name: &str,
    ready: &crate::source::ResolvedSource,
) -> Result<Option<String>> {
    match (request.hold, &ready.commit) {
        (false, _) => Ok(None),
        (true, Some(commit)) => Ok(Some(commit.clone())),
        (true, None) => Err(CoreError::ItemRevUnsupported {
            source_name: source_name.to_owned(),
        }),
    }
}

/// How a source is named in a collision message: its repository or path when
/// the alias is a subscription, the local-source name when it is a fork, and
/// the bare alias as a last resort.
fn source_repo_label(manifest: &Manifest, alias: &str) -> String {
    if alias == crate::manifest::LOCAL_SOURCE_NAME {
        return alias.to_owned();
    }
    manifest
        .sources
        .get(alias)
        .and_then(|decl| decl.repo.clone().or_else(|| decl.path.clone()))
        .unwrap_or_else(|| alias.to_owned())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn declare(
    env: &Env,
    scope: &Scope,
    manifest: &mut Manifest,
    lock: &Lock,
    kind: ItemKind,
    name: &str,
    source_name: &str,
    request: &AddRequest,
    hold_at: Option<&str>,
) -> Result<()> {
    // Invariant 4: same-source redeclare is a no-op; a name already claimed
    // from elsewhere is a hard error naming the original. The claim is either
    // a lock entry (installed) or a manifest declaration not yet applied —
    // both count, or a declared name could be silently rebound to another
    // marketplace, which is exactly the collision the browse view warns about.
    let collision_repo = lock
        .entries
        .values()
        .find(|entry| entry.kind == kind && entry.name == name && entry.source != source_name)
        .map(|entry| entry.source_repo.clone())
        .or_else(|| {
            manifest
                .declared(kind)
                .get(name)
                .filter(|decl| decl.source != source_name)
                .map(|decl| source_repo_label(manifest, &decl.source))
        });
    if let Some(existing) = collision_repo {
        let requested = match source::resolve(env, scope, source_name, manifest)? {
            source::SourceState::Ready(ready) => ready.provenance,
            _ => source_name.to_owned(),
        };
        return Err(CoreError::SourceCollision {
            name: name.to_owned(),
            existing,
            requested,
        });
    }
    let decl = manifest
        .declared_mut(kind)
        .entry(name.to_owned())
        .or_insert_with(|| ItemDecl::from_source(source_name));
    decl.source = source_name.to_owned();
    if let Some(harnesses) = &request.harnesses {
        decl.harnesses = Some(harnesses.clone());
    }
    if request.copy {
        decl.method = Some(Method::Copy);
    }
    if let Some(commit) = hold_at {
        decl.rev = Some(commit.to_owned());
    }
    // Asking for something back is the plainest possible statement that it
    // is wanted, so it outranks a removal recorded earlier.
    if let Some(held) = manifest.suppressed.get_mut(&kind) {
        held.retain(|suppressed| suppressed != name);
    }
    manifest.suppressed.retain(|_, held| !held.is_empty());
    Ok(())
}
