//! Which items an installed bundle brings in.
//!
//! A bundle is a curated set a catalog offers under one name. The manifest
//! records that the set is installed and nothing else — what it holds is the
//! catalog's to say, and it derives here on every plan, each member carrying
//! an edge back to the bundle it came in with. That edge is what lets the
//! bundle be uninstalled later without taking anything the user also asked
//! for, and without stranding anything they did not.
//!
//! Members are the catalog's own items, always: a set cannot reach into
//! another source, because a bare name from somewhere else names nothing
//! stable. A member this catalog does not offer is a finding that says which
//! member, and the rest of the set still installs.
//!
//! Two sets can carry one member and ask for it differently. The tools are
//! simply both, and a set that is switched on installs its member switched
//! on — an unrelated set that is switched off must never be the reason an
//! installed set's own member arrives dead. What is left is a genuine
//! disagreement, so it is reported rather than settled by whichever set the
//! manifest happens to name first.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use crate::lock::{BundleRef, Reason};
use crate::manifest::{ItemDecl, Manifest};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::find_item;

use super::ItemWarning;
use super::desired::{DesiredState, target_harnesses};
use super::expansion::{Catalogs, Expansion};

/// One member, as every set that carries it asked for it.
struct Carried {
    decl: ItemDecl,
    /// The set whose answer stands where the sets disagree.
    by: String,
    /// The edge each set adds, against the tools that set installs on.
    edges: Vec<(Reason, Vec<HarnessId>)>,
}

pub(super) fn expand(
    scope: &Scope,
    manifest: &Manifest,
    expansion: &mut Expansion,
    catalogs: &mut Catalogs,
    state: &mut DesiredState,
) {
    let mut carried: BTreeMap<(ItemKind, String), Carried> = BTreeMap::new();
    for (name, decl) in &manifest.bundles {
        for (kind, member, member_decl, harnesses) in
            installable(name, decl, scope, manifest, catalogs, state)
        {
            let edge = (
                Reason::MemberOf {
                    bundle: BundleRef {
                        source: decl.source.clone(),
                        name: name.clone(),
                        scope: scope.clone(),
                    },
                },
                harnesses,
            );
            match carried.entry((kind, member.clone())) {
                Entry::Vacant(slot) => {
                    slot.insert(Carried {
                        decl: member_decl,
                        by: name.clone(),
                        edges: vec![edge],
                    });
                }
                Entry::Occupied(mut slot) => {
                    let held = slot.get_mut();
                    if let Some(warning) =
                        disagreement(manifest, kind, &member, held, name, &member_decl)
                    {
                        state.warnings.push(warning);
                    }
                    held.decl.enabled |= member_decl.enabled;
                    held.edges.push(edge);
                }
            }
        }
    }
    for ((kind, name), Carried { decl, edges, .. }) in carried {
        for (reason, harnesses) in edges {
            for harness in harnesses {
                expansion.add(kind, &name, &decl, harness, reason.clone());
            }
        }
    }
}

/// The members of one set this plan can actually install, each with the
/// declaration it installs under and the tools it lands on. Every member left
/// out is accounted for: held back by a removal, not offered by the catalog,
/// or of a kind no tool here holds.
fn installable(
    name: &str,
    decl: &ItemDecl,
    scope: &Scope,
    manifest: &Manifest,
    catalogs: &mut Catalogs,
    state: &mut DesiredState,
) -> Vec<(ItemKind, String, ItemDecl, Vec<HarnessId>)> {
    let Some((sealed, config)) = catalogs.get(&decl.source, decl.rev.as_deref(), state) else {
        return Vec::new();
    };
    let Ok(offered) = crate::source::bundles::find(sealed, config, name) else {
        return Vec::new();
    };
    let Some(bundle) = offered else {
        state.notes.push(format!(
            "bundle {name}: the catalog '{}' offers no set by that name",
            decl.source
        ));
        return Vec::new();
    };
    let mut installable = Vec::new();
    let mut held_back = 0;
    for member in &bundle.members {
        // A member the user took away stays away: the bundle is still
        // installed, and the audit says how much of it is not. A member they
        // declared by name is not held back at all — the declaration
        // outranks the record of the removal.
        if manifest.is_held_back(member.kind, &member.name) {
            held_back += 1;
            continue;
        }
        if find_item(sealed, config, member.kind, &member.name).is_none() {
            state.warnings.push(ItemWarning {
                kind: member.kind,
                name: member.name.clone(),
                harness: None,
                message: format!(
                    "the bundle {name} carries {}, which the catalog '{}' does not offer",
                    member.name, decl.source
                ),
                remediation: Some(format!(
                    "add {} to that catalog, or drop it from the bundle {name}",
                    member.name
                )),
            });
            continue;
        }
        // A member installs the way the bundle does: same source, same
        // tools, same method, same held revision, and off while the bundle
        // is off.
        let member_decl = ItemDecl {
            source: decl.source.clone(),
            harnesses: decl.harnesses.clone(),
            method: decl.method,
            rev: decl.rev.clone(),
            enabled: decl.enabled,
        };
        let harnesses = target_harnesses(&member_decl, manifest, member.kind, scope);
        if harnesses.is_empty() {
            state.notes.push(format!(
                "bundle {name}: no tool here holds a {}, so {} was not installed",
                member.kind.name(),
                member.name
            ));
            continue;
        }
        installable.push((member.kind, member.name.clone(), member_decl, harnesses));
    }
    if held_back > 0 {
        state.notes.push(format!(
            "bundle {name}: installed, {held_back} member{} held back",
            match held_back {
                1 => "",
                _ => "s",
            }
        ));
    }
    installable
}

/// What two sets carrying one member cannot agree on, once the tools and the
/// on/off state have been merged. Where it comes from and how it lands are
/// one answer each, so the first set's stands and the user is told they had
/// a choice to make — declaring the item is how they make it.
fn disagreement(
    manifest: &Manifest,
    kind: ItemKind,
    name: &str,
    held: &Carried,
    second: &str,
    theirs: &ItemDecl,
) -> Option<ItemWarning> {
    let method = |decl: &ItemDecl| decl.method.unwrap_or(manifest.install.method);
    let mut differ = Vec::new();
    if held.decl.source != theirs.source {
        differ.push("which catalog it comes from");
    }
    if method(&held.decl) != method(theirs) {
        differ.push("how it is installed");
    }
    if differ.is_empty() {
        return None;
    }
    Some(ItemWarning {
        kind,
        name: name.to_owned(),
        harness: None,
        message: format!(
            "the bundles {} and {second} both carry {name} and disagree about {} — it installs the way {} asks",
            held.by,
            differ.join(" and "),
            held.by
        ),
        remediation: Some(format!(
            "declare the {} {name} in vstack.toml to say how it should install",
            kind.name()
        )),
    })
}

/// The items the record says came in with any of these bundles. A bundle
/// uninstall names them alongside the bundle itself: taking the set away is
/// what takes its members away, and each one goes only if nothing else
/// accounts for it once the bundle's edge is gone.
pub(super) fn recorded_members(lock: &crate::lock::Lock, bundles: &[String]) -> Vec<String> {
    let mut names: Vec<String> = lock
        .entries
        .values()
        .filter(|entry| {
            entry.reasons.iter().any(|reason| match reason {
                Reason::MemberOf { bundle } => bundles.contains(&bundle.name),
                Reason::Requested | Reason::RequiredBy { .. } => false,
            })
        })
        .map(|entry| entry.name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}
