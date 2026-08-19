//! Install-all subsumption, and the one-bundle-per-name rule.
//!
//! Declaring a bundle removes, in the same plan, the individual
//! declarations the bundle now subsumes — otherwise those members keep a
//! `requested` edge and survive a later bundle uninstall as "also
//! requested". Subsumption only claims a declaration whose effective
//! options equal what the bundle derives for that member; one the user
//! shaped — its own harness list, method, hold, enabled flag, frontmatter
//! override or accepted safety decision — is kept, and the preview says
//! why.

use crate::error::{CoreError, Result};
use crate::manifest::{ItemDecl, Manifest};
use crate::model::ItemKind;
use crate::source::CatalogBundle;

/// Invariant 4 for bundles: `[bundles.<name>]` is keyed by bare name, so a
/// second marketplace's same-named bundle is refused naming the first —
/// with installing the members individually as the way out.
pub(super) fn require_free(manifest: &Manifest, name: &str, source_name: &str) -> Result<()> {
    let Some(existing) = manifest.bundles.get(name) else {
        return Ok(());
    };
    if existing.source == source_name {
        return Ok(());
    }
    Err(CoreError::BundleCollision {
        name: name.to_owned(),
        existing: canonical(manifest, &existing.source),
        requested: canonical(manifest, source_name),
    })
}

/// The subscription's canonical repository (or path) beside its alias —
/// an alias is a local label, not an identity.
fn canonical(manifest: &Manifest, alias: &str) -> String {
    match manifest
        .sources
        .get(alias)
        .and_then(|decl| decl.repo.as_deref().or(decl.path.as_deref()))
    {
        Some(repo) => format!("{alias} ({repo})"),
        None => alias.to_owned(),
    }
}

/// Drop the individual declarations this bundle now accounts for, and say
/// so — "N packages now come with the bundle". A member whose declaration
/// differs from what the bundle would derive is kept, with the note naming
/// what the user changed.
pub(super) fn subsume(
    manifest: &mut Manifest,
    bundle: &CatalogBundle,
    bundle_decl: &ItemDecl,
    notes: &mut Vec<String>,
) {
    let mut taken = 0usize;
    for member in &bundle.members {
        let Some(decl) = manifest.declared(member.kind).get(&member.name) else {
            continue;
        };
        if decl.source != bundle_decl.source {
            continue;
        }
        match shaped_by_user(manifest, member.kind, &member.name, decl, bundle_decl) {
            None => {
                manifest.declared_mut(member.kind).remove(&member.name);
                taken += 1;
            }
            Some(why) => notes.push(format!("'{}' stays your own install — {why}", member.name)),
        }
    }
    match taken {
        0 => {}
        1 => notes.push(format!(
            "1 package now comes with the {} bundle",
            bundle.name
        )),
        n => notes.push(format!(
            "{n} packages now come with the {} bundle",
            bundle.name
        )),
    }
}

/// Why a member's own declaration is not the bundle's — `None` when the
/// two are effectively equal and the bundle can speak for it.
fn shaped_by_user(
    manifest: &Manifest,
    kind: ItemKind,
    name: &str,
    decl: &ItemDecl,
    bundle_decl: &ItemDecl,
) -> Option<String> {
    if decl.harnesses != bundle_decl.harnesses {
        return Some("it has its own harness list".to_owned());
    }
    if decl.method != bundle_decl.method {
        return Some("it has its own install method".to_owned());
    }
    if decl.rev != bundle_decl.rev {
        return Some("it is held at its own version".to_owned());
    }
    if decl.enabled != bundle_decl.enabled {
        return Some("you toggled it yourself".to_owned());
    }
    let about_item = |key: &String| {
        crate::lock::parse_entry_key(key)
            .is_some_and(|(key_kind, key_name, _)| key_kind == kind && key_name == name)
    };
    if manifest.safety_overrides.keys().any(about_item)
        || manifest.safety_reviews.keys().any(about_item)
    {
        return Some("it carries safety decisions you accepted".to_owned());
    }
    if kind == ItemKind::Agent
        && manifest
            .agent_frontmatter
            .values()
            .any(|agents| agents.contains_key(name))
    {
        return Some("it carries your frontmatter overrides".to_owned());
    }
    None
}
