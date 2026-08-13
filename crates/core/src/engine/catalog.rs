//! What a catalog's own naming costs the scope reading it: declarations
//! that would land on the same file, and the problems the catalog reports
//! about itself.

use std::collections::{BTreeMap, BTreeSet};

use crate::manifest::Manifest;
use crate::model::{HarnessId, ItemKind, Scope};
use crate::names;
use crate::source::SourceConfig;

use super::desired::{DesiredState, Refused, target_harnesses};

/// The installations two declarations both claim. Namespacing is what makes
/// this reachable: `a/b` and a plain `a__b` are two names in `vstack.toml`
/// and one file on disk, as are two names a filesystem folds together.
/// Neither is installed — writing one would hand its content to the other's
/// name, and there is no way to tell which one the user meant.
pub(super) struct Collisions(BTreeSet<(ItemKind, String, HarnessId)>);

impl Collisions {
    /// Every clash the manifest holds, recorded as refusals on the state.
    pub(super) fn find(manifest: &Manifest, scope: &Scope, state: &mut DesiredState) -> Collisions {
        let mut claimed = BTreeSet::new();
        for (kind, table) in [
            (ItemKind::Skill, &manifest.skills),
            (ItemKind::Agent, &manifest.agents),
            (ItemKind::Command, &manifest.commands),
        ] {
            // Folded rendered name → the declarations that spell it, per
            // tool: the same two names can clash on one tool and not on
            // another, since the tools join a plugin to an item differently.
            let mut claims: BTreeMap<(HarnessId, String), Vec<String>> = BTreeMap::new();
            for (name, decl) in table {
                for harness in target_harnesses(decl, manifest, kind, scope) {
                    let rendered = crate::harness::rendered_name(harness, name);
                    claims
                        .entry((harness, names::fold(&rendered)))
                        .or_default()
                        .push(name.clone());
                }
            }
            for ((harness, _), names) in claims {
                if names.len() < 2 {
                    continue;
                }
                for name in &names {
                    let rendered = crate::harness::rendered_name(harness, name);
                    let others: Vec<&str> = names
                        .iter()
                        .filter(|other| *other != name)
                        .map(String::as_str)
                        .collect();
                    state.refused.push(Refused {
                        kind,
                        name: name.clone(),
                        harness,
                        reason: format!(
                            "`{name}` and `{}` both install as `{rendered}` on {} — one would take the other's place",
                            others.join("`, `"),
                            harness.display_name()
                        ),
                    });
                    claimed.insert((kind, name.clone(), harness));
                }
            }
        }
        Collisions(claimed)
    }

    pub(super) fn allows(&self, kind: ItemKind, name: &str, harness: HarnessId) -> bool {
        !self.0.contains(&(kind, name.to_owned(), harness))
    }
}

/// What the catalog says is wrong with itself, said once per source however
/// many items are read from it.
pub(super) fn notes(config: &SourceConfig, source: &str, state: &mut DesiredState) {
    for finding in config.findings() {
        let note = format!("source '{source}': {finding}");
        if !state.notes.contains(&note) {
            state.notes.push(note);
        }
    }
}
