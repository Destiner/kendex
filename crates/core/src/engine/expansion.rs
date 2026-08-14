//! What a plan installs, and why each installation exists.
//!
//! The manifest holds choices: the items asked for, the bundles installed,
//! which optional dependencies were taken, what stays removed. Here those
//! choices become the whole set — bundle members and skill dependencies
//! included — with a reason edge on every installation. None of it is written
//! back. An item that arrived as a member or a dependency must never read as
//! one the user asked for, or removing whatever brought it in could never
//! take it away again.

use std::collections::{BTreeMap, BTreeSet};

use crate::env::Env;
use crate::lock::Reason;
use crate::manifest::{ItemDecl, Manifest};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::{SourceConfig, SourceState, source_config};
use crate::source_read::SealedSource;

use super::desired::{DesiredState, target_harnesses};

/// The kinds a plan installs, in the order it plans them.
pub(super) const PLANNED_KINDS: [ItemKind; 5] = [
    ItemKind::Skill,
    ItemKind::Agent,
    ItemKind::Hook,
    ItemKind::Command,
    ItemKind::McpServer,
];

/// One item a plan installs: the declaration to plan it under, and the tools
/// it lands on. A declared item keeps the declaration the user wrote; a
/// derived one gets its source from whatever brought it in.
pub(super) struct Planned {
    pub(super) decl: ItemDecl,
    pub(super) harnesses: Vec<HarnessId>,
}

#[derive(Default)]
pub(super) struct Expansion {
    items: BTreeMap<(ItemKind, String), Planned>,
    reasons: BTreeMap<(ItemKind, String, HarnessId), BTreeSet<Reason>>,
}

impl Expansion {
    /// Everything of one kind this plan installs, in name order.
    pub(super) fn of(&self, kind: ItemKind) -> Vec<(&String, &Planned)> {
        self.items
            .iter()
            .filter(|((of_kind, _), _)| *of_kind == kind)
            .map(|((_, name), planned)| (name, planned))
            .collect()
    }

    pub(super) fn reasons(
        &self,
        kind: ItemKind,
        name: &str,
        harness: HarnessId,
    ) -> BTreeSet<Reason> {
        self.reasons
            .get(&(kind, name.to_owned(), harness))
            .cloned()
            .unwrap_or_default()
    }

    pub(super) fn contains(&self, kind: ItemKind, name: &str) -> bool {
        self.items.contains_key(&(kind, name.to_owned()))
    }

    /// The catalog an item in this expansion installs from.
    pub(super) fn source_of(&self, kind: ItemKind, name: &str) -> Option<String> {
        self.items
            .get(&(kind, name.to_owned()))
            .map(|planned| planned.decl.source.clone())
    }

    pub(super) fn harnesses(&self, kind: ItemKind, name: &str) -> Vec<HarnessId> {
        self.items
            .get(&(kind, name.to_owned()))
            .map(|planned| planned.harnesses.clone())
            .unwrap_or_default()
    }

    /// A declaration the user wrote: it installs as written, and it is here
    /// even when no tool can hold it — the plan says so rather than going
    /// quiet about a declaration that produced nothing.
    fn declared(&mut self, kind: ItemKind, name: &str, decl: &ItemDecl, harnesses: Vec<HarnessId>) {
        for harness in &harnesses {
            self.reasons
                .entry((kind, name.to_owned(), *harness))
                .or_default()
                .insert(Reason::Requested);
        }
        self.items.insert(
            (kind, name.to_owned()),
            Planned {
                decl: decl.clone(),
                harnesses,
            },
        );
    }

    /// Record one derived reason, returning whether this taught the expansion
    /// something new — which is what keeps a cycle from walking forever.
    pub(super) fn add(
        &mut self,
        kind: ItemKind,
        name: &str,
        decl: &ItemDecl,
        harness: HarnessId,
        reason: Reason,
    ) -> bool {
        let fresh = self
            .reasons
            .entry((kind, name.to_owned(), harness))
            .or_default()
            .insert(reason);
        let planned = self
            .items
            .entry((kind, name.to_owned()))
            .or_insert_with(|| Planned {
                decl: decl.clone(),
                harnesses: Vec::new(),
            });
        if !planned.harnesses.contains(&harness) {
            planned.harnesses.push(harness);
        }
        fresh
    }
}

/// Every catalog read this pass, opened once. Sources that cannot be read
/// carry nothing to derive; the declaration that names one reports that on
/// its own, where it can say which declaration it cost.
pub(super) struct Catalogs<'a> {
    env: &'a Env,
    scope: &'a Scope,
    manifest: &'a Manifest,
    open: BTreeMap<String, Option<(SealedSource, SourceConfig)>>,
}

impl Catalogs<'_> {
    pub(super) fn get(
        &mut self,
        source: &str,
        state: &mut DesiredState,
    ) -> Option<&(SealedSource, SourceConfig)> {
        if !self.open.contains_key(source) {
            let opened = self.read(source, state);
            self.open.insert(source.to_owned(), opened);
        }
        self.open.get(source).and_then(Option::as_ref)
    }

    fn read(&self, source: &str, state: &mut DesiredState) -> Option<(SealedSource, SourceConfig)> {
        let resolution = match state.sources.get(source) {
            Some(resolution) => resolution.clone(),
            None => {
                let resolution =
                    crate::source::resolve(self.env, self.scope, source, self.manifest).ok()?;
                state.sources.insert(source.to_owned(), resolution.clone());
                resolution
            }
        };
        let SourceState::Ready(ready) = resolution else {
            return None;
        };
        let sealed = SealedSource::open(&ready.root).ok()?;
        let config = source_config(&sealed).ok()?;
        Some((sealed, config))
    }
}

/// The whole installed set this manifest asks for: what it declares, what the
/// bundles it installs carry, and what those skills require.
pub(super) fn expand(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    state: &mut DesiredState,
) -> Expansion {
    let mut expansion = Expansion::default();
    for kind in PLANNED_KINDS {
        for (name, decl) in manifest.declared(kind) {
            let harnesses = target_harnesses(decl, manifest, kind, scope);
            expansion.declared(kind, name, decl, harnesses);
            // A removal is recorded so that nothing derives the item back on
            // its own. Declaring it by name is the plainest statement that it
            // is wanted, so it installs and the record sits there doing
            // nothing — one of the two has to go, and the user picks which.
            if manifest.is_suppressed(kind, name) {
                state.notes.push(format!(
                    "{} {name} is declared and also kept removed — the declaration wins and it installs; drop it from [suppressed] in vstack.toml to settle it",
                    kind.name()
                ));
            }
        }
    }
    let mut catalogs = Catalogs {
        env,
        scope,
        manifest,
        open: BTreeMap::new(),
    };
    super::bundles::expand(scope, manifest, &mut expansion, &mut catalogs, state);
    super::deps::expand(scope, manifest, &mut expansion, &mut catalogs, state);
    expansion
}
