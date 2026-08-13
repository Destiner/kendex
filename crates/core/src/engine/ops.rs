use super::{EngineReport, PlanOptions, plan_scope};
use crate::env::Env;
use crate::error::Result;
use crate::lock::lock_path;
use crate::manifest::{self, Manifest};
use crate::model::{HarnessId, ItemKind, Scope};

mod add;
mod persist;
pub use add::{AddRequest, add};
use persist::ensure_manifest_persisted;

/// Every kind a manifest declares by name. Plugins are excluded: they carry
/// only an enabled flag, in their own table.
const DECLARED_KINDS: [ItemKind; 6] = [
    ItemKind::Agent,
    ItemKind::Skill,
    ItemKind::Hook,
    ItemKind::Command,
    ItemKind::McpServer,
    ItemKind::PiExtension,
];

/// The tools on this machine a fresh manifest should install to — a tool
/// vstack can only read is detected and listed, never seeded as a target
/// whose every install would silently do nothing.
fn detected_harnesses(env: &Env) -> Vec<HarnessId> {
    crate::harness::all_adapters()
        .iter()
        .filter_map(|a| {
            a.detect(env, &a.default_global_root(env))
                .map(|found| found.harness)
        })
        .filter(|harness| crate::harness::installable(*harness))
        .collect()
}

/// Load the scope's manifest for mutation, seeding a fresh one (with the
/// default source) when none exists. Legacy files are a hard error.
pub fn manifest_for_mutation(env: &Env, scope: &Scope) -> Result<Manifest> {
    let path = manifest::manifest_path(env, scope);
    match manifest::load_for_mutation(&path)? {
        Some(manifest) => Ok(manifest),
        None => Ok(manifest::seed(&detected_harnesses(env))),
    }
}

/// Drop declarations and plan the removal of exactly those items. A removal
/// is durable: an item something else still requires is written down as
/// suppressed rather than re-derived on the next plan, and every item that
/// requires it says so in the audit instead of quietly getting it back.
/// `sweep` also removes what nothing needs anymore — the dependencies whose
/// last dependent is going away.
///
/// A name that is an installed bundle removes the set: its members go with
/// it, except the ones the user also asked for, that a surviving item needs,
/// or that another installed bundle carries too.
pub fn remove(env: &Env, scope: &Scope, names: &[String], sweep: bool) -> Result<EngineReport> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    let lock = crate::lock::load(&lock_path(env, scope))?;
    let bundles: Vec<String> = names
        .iter()
        .filter(|name| manifest.bundles.contains_key(*name))
        .cloned()
        .collect();
    let mut removing = names.to_vec();
    removing.extend(super::bundles::recorded_members(&lock, &bundles));
    for name in names {
        manifest.bundles.remove(name);
        for kind in DECLARED_KINDS {
            manifest.declared_mut(kind).remove(name);
        }
        manifest.plugins.remove(name);
        manifest.agent_skills.remove(name);
        manifest.skill_instructions.remove(name);
        manifest.optional_dependencies.remove(name);
        // Taking an item away also un-takes it wherever it was chosen as an
        // optional extra: that choice is the whole reason it would return.
        for taken in manifest.optional_dependencies.values_mut() {
            taken.retain(|chosen| chosen != name);
        }
    }
    manifest.optional_dependencies.retain(|_, t| !t.is_empty());
    for (kind, name) in still_derived(env, scope, &manifest, names) {
        manifest.suppress(kind, &name);
    }
    let options = PlanOptions {
        remove_orphans: true,
        removal_filter: Some(removing),
        sweep_unneeded: sweep,
        uninstalled_bundles: bundles,
    };
    let mut report = plan_scope(env, scope, &manifest, &lock, &options)?;
    ensure_manifest_persisted(env, scope, &manifest, &mut report)?;
    Ok(report)
}

/// Which of these names something that stays would pull straight back in,
/// and as what: a dependency of a skill that stays, or a member of a bundle
/// that is still installed. Those are the removals that have to be written
/// down, or the next plan would simply undo them.
fn still_derived(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    names: &[String],
) -> Vec<(ItemKind, String)> {
    let mut state = crate::engine::desired::DesiredState::default();
    let expansion = super::expansion::expand(env, scope, manifest, &mut state);
    let mut derived = Vec::new();
    for name in names {
        for kind in super::expansion::PLANNED_KINDS {
            if expansion.contains(kind, name) {
                derived.push((kind, name.clone()));
            }
        }
    }
    derived
}

/// Flip declarations; disabling is non-destructive (invariant 5).
pub fn toggle(env: &Env, scope: &Scope, names: &[String], enabled: bool) -> Result<EngineReport> {
    let mut manifest = manifest_for_mutation(env, scope)?;
    let lock = crate::lock::load(&lock_path(env, scope))?;
    for name in names {
        for kind in DECLARED_KINDS {
            if let Some(decl) = manifest.declared_mut(kind).get_mut(name) {
                decl.enabled = enabled;
            }
        }
        if let Some(plugin) = manifest.plugins.get_mut(name) {
            plugin.enabled = enabled;
        }
    }
    let mut report = plan_scope(env, scope, &manifest, &lock, &PlanOptions::default())?;
    ensure_manifest_persisted(env, scope, &manifest, &mut report)?;
    Ok(report)
}
