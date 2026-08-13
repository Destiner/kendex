//! Which items are installed because another item requires them.
//!
//! Dependencies are declared in an item's own frontmatter, the way v1
//! declared them: a `dependencies` map holding `required` and `optional`
//! lists of bare names. Bare names are why the relation stays inside one
//! catalog and one kind — a catalog author cannot know what a consumer
//! calls their sources, so a name from somewhere else has no stable
//! identity to point at. Curation across catalogs and kinds is what bundles
//! are for.
//!
//! Nothing here is written to the manifest. The manifest records choices —
//! what was asked for, which optional dependencies were taken, what stays
//! removed — and this module derives the closure again on every plan, so an
//! item that arrived as a dependency never reads as one the user asked for.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::env::Env;
use crate::error::Result;
use crate::lock::{InstallRef, Reason};
use crate::manifest::{ItemDecl, Manifest};
use crate::model::{HarnessId, ItemKind, Scope};
use crate::source::{SourceConfig, SourceState, find_item, list_items, source_config};
use crate::source_read::SealedSource;

use super::ItemWarning;
use super::desired::{DesiredState, target_harnesses};

/// One item's declared dependencies. Names are as the author wrote them.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Dependencies {
    pub(crate) required: Vec<String>,
    pub(crate) optional: Vec<String>,
}

/// What a skill's frontmatter declares it needs. Read through the bounded
/// parser every other frontmatter read goes through; there is no fallback to
/// scanning the body, because a dependency the author never declared is not
/// a dependency. A block that will not parse is left to the renderer, which
/// reads the same bytes and reports what is wrong with them.
pub(crate) fn declared_dependencies(
    sealed: &SealedSource,
    skill_dir: &std::path::Path,
) -> Result<Dependencies> {
    let Some(text) = sealed.read_if_exists(&skill_dir.join("SKILL.md"))? else {
        return Ok(Dependencies::default());
    };
    let Ok((yaml, _)) = crate::frontmatter::split(&text) else {
        return Ok(Dependencies::default());
    };
    let Ok(parsed) = crate::frontmatter::parse_tolerant(yaml) else {
        return Ok(Dependencies::default());
    };
    let Some(crate::frontmatter::Value::Map(map)) = parsed.map.get("dependencies") else {
        return Ok(Dependencies::default());
    };
    Ok(Dependencies {
        required: map.string_list("required").unwrap_or_default(),
        optional: map.string_list("optional").unwrap_or_default(),
    })
}

/// The skills a plan installs and why each installation exists — the
/// declared ones plus everything they require, keyed the way the lock keys
/// an installation.
#[derive(Debug, Default)]
pub(super) struct Expansion {
    /// The declaration to plan each skill under. A derived one inherits its
    /// parent's source and carries the harnesses its parents need it on.
    pub(super) items: BTreeMap<String, ItemDecl>,
    reasons: BTreeMap<(String, HarnessId), BTreeSet<Reason>>,
}

impl Expansion {
    pub(super) fn reasons(&self, name: &str, harness: HarnessId) -> BTreeSet<Reason> {
        self.reasons
            .get(&(name.to_owned(), harness))
            .cloned()
            .unwrap_or_default()
    }

    /// Record one reason, returning whether this taught the expansion
    /// something new — which is what keeps a cycle from walking forever.
    fn add(&mut self, name: &str, decl: &ItemDecl, harness: HarnessId, reason: Reason) -> bool {
        let fresh = self
            .reasons
            .entry((name.to_owned(), harness))
            .or_default()
            .insert(reason);
        let entry = self
            .items
            .entry(name.to_owned())
            .or_insert_with(|| ItemDecl {
                harnesses: Some(Vec::new()),
                ..decl.clone()
            });
        let harnesses = entry.harnesses.get_or_insert_with(Vec::new);
        if !harnesses.contains(&harness) {
            harnesses.push(harness);
        }
        fresh
    }

    fn harnesses(&self, name: &str) -> Vec<HarnessId> {
        self.items
            .get(name)
            .and_then(|decl| decl.harnesses.clone())
            .unwrap_or_default()
    }
}

/// Every catalog read this pass, opened once. Sources that cannot be read
/// carry no dependencies to find; the declaration that names one reports
/// that on its own, where it can say which declaration it cost.
struct Catalogs<'a> {
    env: &'a Env,
    scope: &'a Scope,
    manifest: &'a Manifest,
    open: BTreeMap<String, Option<(SealedSource, SourceConfig)>>,
}

impl Catalogs<'_> {
    fn get(
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

/// The declared skills plus everything they require, walked until no
/// installation learns a new reason. Cycles are fine — v1's `orch` and `dev`
/// require each other on purpose — because an item is only walked again when
/// its reasons grow, and they cannot grow forever.
pub(super) fn expand(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    state: &mut DesiredState,
) -> Expansion {
    let mut expansion = Expansion::default();
    let mut queue: VecDeque<String> = VecDeque::new();
    for (name, decl) in &manifest.skills {
        for harness in target_harnesses(decl, manifest, ItemKind::Skill, scope) {
            expansion.add(name, decl, harness, Reason::Requested);
        }
        queue.push_back(name.clone());
    }
    let mut catalogs = Catalogs {
        env,
        scope,
        manifest,
        open: BTreeMap::new(),
    };
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    while let Some(parent) = queue.pop_front() {
        // A declaration no tool here can hold installs nothing, so it needs
        // nothing either; the declaration itself reports that.
        let Some(source) = expansion.items.get(&parent).map(|decl| decl.source.clone()) else {
            continue;
        };
        for (dep, harnesses) in wanted_by(&parent, &expansion, manifest, &mut catalogs, state) {
            edges.entry(parent.clone()).or_default().insert(dep.clone());
            let decl = ItemDecl {
                source: source.clone(),
                harnesses: Some(Vec::new()),
                // A derived installation takes the scope's own default
                // method: its parent's is a choice about the parent.
                method: None,
                enabled: true,
            };
            let mut grew = false;
            for harness in harnesses {
                let by = InstallRef {
                    source: decl.source.clone(),
                    kind: ItemKind::Skill,
                    name: parent.clone(),
                    harness,
                    scope: scope.clone(),
                };
                grew |= expansion.add(&dep, &decl, harness, Reason::RequiredBy { by });
            }
            if grew {
                queue.push_back(dep);
            }
        }
    }
    for members in cycles(&edges) {
        state.notes.push(format!(
            "skills {} require each other — all of them install",
            members.join(" and ")
        ));
    }
    expansion
}

/// One item's dependencies, resolved against its own catalog: the required
/// ones, plus the optional ones this manifest chose. Everything that cannot
/// be resolved is a warning on the item that asked for it — a dependency is
/// never dropped in silence.
fn wanted_by(
    parent: &str,
    expansion: &Expansion,
    manifest: &Manifest,
    catalogs: &mut Catalogs,
    state: &mut DesiredState,
) -> Vec<(String, Vec<HarnessId>)> {
    let Some(decl) = expansion.items.get(parent).cloned() else {
        return Vec::new();
    };
    let harnesses = expansion.harnesses(parent);
    let Some((sealed, config)) = catalogs.get(&decl.source, state) else {
        return Vec::new();
    };
    let Some(dir) = find_item(sealed, config, ItemKind::Skill, parent) else {
        return Vec::new();
    };
    let Ok(declared) = declared_dependencies(sealed, &dir) else {
        return Vec::new();
    };
    let chosen = manifest
        .optional_dependencies
        .get(parent)
        .cloned()
        .unwrap_or_default();
    for name in chosen.iter().filter(|c| !declared.optional.contains(c)) {
        state.warnings.push(warn(
            parent,
            format!("{name} was chosen as an optional dependency, and {parent} does not offer one by that name"),
            format!("remove {name} from optional-dependencies.{parent} in vstack.toml"),
        ));
    }
    let mut wanted = Vec::new();
    for name in declared
        .required
        .iter()
        .chain(declared.optional.iter().filter(|o| chosen.contains(o)))
    {
        let Some(dep) = resolve(name, parent, sealed, config, &decl.source, state) else {
            continue;
        };
        if manifest.is_suppressed(ItemKind::Skill, &dep) {
            state.warnings.push(warn(
                parent,
                format!("missing required dependency: {parent} requires {dep}, which is kept removed"),
                format!("add the skill {dep} again to restore it, or drop it from {parent}'s dependencies"),
            ));
            continue;
        }
        wanted.push((
            dep.clone(),
            for_harnesses(&dep, parent, &harnesses, manifest, state),
        ));
    }
    wanted
}

/// Where a bare dependency name points inside its own catalog. A name the
/// catalog does not carry, or carries more than once under different
/// plugins, is a finding naming what it found.
fn resolve(
    name: &str,
    parent: &str,
    sealed: &SealedSource,
    config: &SourceConfig,
    source: &str,
    state: &mut DesiredState,
) -> Option<String> {
    if find_item(sealed, config, ItemKind::Skill, name).is_some() {
        return Some(name.to_owned());
    }
    let candidates: Vec<String> = list_items(sealed, config, ItemKind::Skill)
        .into_iter()
        .filter(|offered| offered.rsplit('/').next() == Some(name))
        .collect();
    match candidates.len() {
        1 => candidates.into_iter().next(),
        0 => {
            state.warnings.push(warn(
                parent,
                format!("{parent} requires {name}, which the catalog '{source}' does not offer"),
                format!("add {name} to that catalog, or drop it from {parent}'s dependencies"),
            ));
            None
        }
        _ => {
            state.warnings.push(warn(
                parent,
                format!(
                    "{parent} requires {name}, and the catalog '{source}' offers {}",
                    candidates.join(" and ")
                ),
                format!("name one of them in full in {parent}'s dependencies"),
            ));
            None
        }
    }
}

/// The tools a dependency installs for: the ones its parent needs it on,
/// narrowed by what the dependency's own declaration allows and by the tools
/// that can hold a skill here. A tool left out is a warning on the parent —
/// it will run without something it says it needs — never a block.
fn for_harnesses(
    dep: &str,
    parent: &str,
    parent_harnesses: &[HarnessId],
    manifest: &Manifest,
    state: &mut DesiredState,
) -> Vec<HarnessId> {
    let own = manifest.skills.get(dep).and_then(|d| d.harnesses.clone());
    let installs: Vec<HarnessId> = parent_harnesses
        .iter()
        .copied()
        .filter(|harness| own.as_ref().is_none_or(|list| list.contains(harness)))
        .collect();
    let missing: Vec<&str> = parent_harnesses
        .iter()
        .filter(|harness| !installs.contains(harness))
        .map(|harness| harness.display_name())
        .collect();
    if !missing.is_empty() {
        state.warnings.push(warn(
            parent,
            format!(
                "missing required dependency: {} {} {parent} without {dep}, which it requires",
                missing.join(" and "),
                match missing.len() {
                    1 => "runs",
                    _ => "run",
                }
            ),
            format!("declare {dep} for {} too", missing.join(" and ")),
        ));
    }
    installs
}

fn warn(name: &str, message: String, remediation: String) -> ItemWarning {
    ItemWarning {
        kind: ItemKind::Skill,
        name: name.to_owned(),
        harness: None,
        message,
        remediation: Some(remediation),
    }
}

mod graph;
use graph::cycles;
