//! Unsubscribing: what leaves with a marketplace, and removing it.
//!
//! The closure of a source is **derived by re-expansion, not read off
//! declarations**: expand the installed set with the source present and again
//! with its declarations gone, and diff. A derived dependency never names the
//! source in the manifest, so only the difference between the two expansions
//! tells the truth about what its going takes with it. When the source bytes
//! that expansion needs are unreachable, the closure refuses rather than infer.

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::{ItemDecl, Manifest};
use crate::model::{ItemKind, Scope};

use super::EngineReport;
use super::planned::planned_declarations;

/// One item that leaves with the source: its kind and name, the declaration it
/// installs under, and whether it was derived (a bundle member or a dependency)
/// rather than declared by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureItem {
    pub kind: ItemKind,
    pub name: String,
    pub decl: ItemDecl,
    pub derived: bool,
}

/// Everything a source's going removes: the items (declared and derived) and
/// the curated sets declared from it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Closure {
    pub items: Vec<ClosureItem>,
    pub bundles: Vec<String>,
}

/// The closure of a subscription, by re-expansion. Refuses when the source is
/// not readable at the commit its installations were expanded from — a closure
/// inferred from an unreachable catalog could strand a derived dependency or
/// sweep one that another parent still keeps.
pub fn closure(
    env: &Env,
    scope: &Scope,
    source_name: &str,
    manifest: &Manifest,
) -> Result<Closure> {
    let scope = scope.canonical();
    if !manifest.sources.contains_key(source_name) {
        return Err(CoreError::UnknownSource {
            name: source_name.to_owned(),
        });
    }
    // The expansion reads the source's bundles and dependencies; if it cannot
    // be reached, refuse and name the fix rather than compute a wrong closure.
    match crate::source::resolve(env, &scope, source_name, manifest)? {
        crate::source::SourceState::Ready(_) => {}
        crate::source::SourceState::Pending { .. } => {
            return Err(CoreError::SourcePending {
                name: source_name.to_owned(),
            });
        }
        crate::source::SourceState::Disabled { .. } => {
            return Err(CoreError::SourceDisabled {
                name: source_name.to_owned(),
            });
        }
        crate::source::SourceState::Missing { path, .. } => {
            return Err(CoreError::SourceMissing {
                name: source_name.to_owned(),
                path,
            });
        }
    }

    let before = planned_declarations(env, &scope, manifest);
    let without = without_source(manifest, source_name);
    let after = planned_declarations(env, &scope, &without);

    let kept: std::collections::BTreeSet<(ItemKind, String)> = after
        .iter()
        .map(|item| (item.kind, item.name.clone()))
        .collect();
    let items = before
        .into_iter()
        .filter(|item| !kept.contains(&(item.kind, item.name.clone())))
        .map(|item| ClosureItem {
            kind: item.kind,
            name: item.name,
            decl: item.decl,
            derived: item.derived,
        })
        .collect();
    let bundles = manifest
        .bundles
        .iter()
        .filter(|(_, decl)| decl.source == source_name)
        .map(|(name, _)| name.clone())
        .collect();
    Ok(Closure { items, bundles })
}

/// The manifest with every declaration that names the source dropped — items,
/// bundles, and the source itself — the post-mutation half of the diff.
fn without_source(manifest: &Manifest, source_name: &str) -> Manifest {
    let mut out = manifest.clone();
    for kind in super::expansion::PLANNED_KINDS {
        out.declared_mut(kind)
            .retain(|_, decl| decl.source != source_name);
    }
    out.bundles.retain(|_, decl| decl.source != source_name);
    out.sources.remove(source_name);
    // An optional-dependency choice whose parent skill is gone is gone too.
    out.optional_dependencies
        .retain(|parent, _| out.skills.contains_key(parent));
    out
}

/// Unsubscribe and uninstall: remove every declaration the source's closure
/// covers, then let the plan sweep the installations and any dependency whose
/// only parents left with it. Members another marketplace's bundle still
/// carries stay, by the same edge rules an ordinary removal follows.
pub fn remove(env: &Env, scope: &Scope, source_name: &str) -> Result<EngineReport> {
    let manifest = super::ops::manifest_for_mutation(env, scope)?;
    // Validate reachability the same way the closure does, so remove and its
    // preview never disagree about whether the source can be read.
    let closure = closure(env, scope, source_name, &manifest)?;
    let without = without_source(&manifest, source_name);
    // Dropping the declarations orphans their installations; remove takes those
    // off disk. The filter is the closure's own names, so orphan removal is
    // scoped to what left with this source — a derived dependency is named so
    // it is not kept as "unaccountable" now that its origin is gone — and no
    // unrelated pre-existing orphan is swept along.
    let lock = crate::lock::load(&crate::lock::lock_path(env, scope))?;
    let options = super::PlanOptions {
        remove_orphans: true,
        removal_filter: Some(closure.items.iter().map(|i| i.name.clone()).collect()),
        sweep_unneeded: true,
        ..super::PlanOptions::default()
    };
    let mut report = super::plan_scope(env, scope, &without, &lock, &options)?;
    if !super::persists_manifest(&report.plan.ops) {
        crate::rename::insert_manifest_save(env, scope, &mut report.plan, without)?;
    }
    Ok(report)
}
