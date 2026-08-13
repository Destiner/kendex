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

use crate::lock::{BundleRef, Reason};
use crate::manifest::{ItemDecl, Manifest};
use crate::model::Scope;
use crate::source::find_item;

use super::ItemWarning;
use super::desired::{DesiredState, target_harnesses};
use super::expansion::{Catalogs, Expansion};

pub(super) fn expand(
    scope: &Scope,
    manifest: &Manifest,
    expansion: &mut Expansion,
    catalogs: &mut Catalogs,
    state: &mut DesiredState,
) {
    for (name, decl) in &manifest.bundles {
        let Some((sealed, config)) = catalogs.get(&decl.source, state) else {
            continue;
        };
        let Ok(offered) = crate::source::bundles::find(sealed, config, name) else {
            continue;
        };
        let Some(bundle) = offered else {
            state.notes.push(format!(
                "bundle {name}: the catalog '{}' offers no set by that name",
                decl.source
            ));
            continue;
        };
        let mut held_back = 0;
        for member in &bundle.members {
            // A member the user took away stays away: the bundle is still
            // installed, and the audit says how much of it is not.
            if manifest.is_suppressed(member.kind, &member.name) {
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
            // tools, same method, and off while the bundle is off.
            let member_decl = ItemDecl {
                source: decl.source.clone(),
                harnesses: decl.harnesses.clone(),
                method: decl.method,
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
            let reason = Reason::MemberOf {
                bundle: BundleRef {
                    source: decl.source.clone(),
                    name: name.clone(),
                    scope: scope.clone(),
                },
            };
            for harness in harnesses {
                expansion.add(
                    member.kind,
                    &member.name,
                    &member_decl,
                    harness,
                    reason.clone(),
                );
            }
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
    }
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
