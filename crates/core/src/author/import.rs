//! The Import wizard's core: every package on this machine as a candidate,
//! and the previewed copy that brings chosen ones into an authored catalog.
//!
//! One inventory, keyed by `(kind, name)`, every byte origin listed.
//! Provenance decides the group: the person's own local-source content,
//! marketplace content (whose licence gates the copy), and unmanaged
//! on-disk content captured as-is. Nothing is guessed: unknown licence
//! blocks marketplace-origin copying unless the person states a basis, a
//! moved origin refuses at apply, and a destination collision — byte or
//! case-fold — is a refusal naming both sides.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::library::Origin;
use crate::manifest::{LOCAL_SOURCE_NAME, Manifest, ManifestFile};
use crate::model::{ItemKind, Scope};
use crate::source_read::SealedSource;

/// One importable package, with every byte origin that offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidate {
    pub kind: ItemKind,
    pub name: String,
    /// Why a harness would refuse this name, when one would — the wizard
    /// requires a different destination name then.
    pub name_problem: Option<String>,
    /// Distinct byte origins, presentation-ordered own → marketplace →
    /// unmanaged. Identical bytes collapse to one entry listing every
    /// location; differing bytes stay separate for the person to choose.
    pub origins: Vec<CandidateOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CandidateOrigin {
    pub group: CandidateGroup,
    /// Every place these exact bytes were seen.
    pub locations: Vec<String>,
    /// Content identity — what apply revalidates before copying.
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "group",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum CandidateGroup {
    /// The person's own content in a local source.
    Own,
    /// Copied from a subscribed marketplace; its licence is shown and the
    /// person confirms they may republish.
    Marketplace {
        source: String,
        repo: String,
        license: Option<String>,
    },
    /// On disk, managed by nothing — captured the way adopt captures.
    Unmanaged,
}

/// What the wizard chose for one candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSelection {
    pub kind: ItemKind,
    /// The inventory name the bytes are found under.
    pub name: String,
    /// The name to write into the catalog — the inventory name unless a
    /// harness would refuse it.
    pub destination: String,
    /// Which bytes: the chosen origin's hash.
    pub hash: String,
    /// Marketplace-origin only: the person confirms the shown licence
    /// permits republishing.
    #[serde(default)]
    pub license_confirmed: bool,
    /// Marketplace-origin with no detectable licence: the person's stated
    /// basis for copying ("author granted permission", say). Never
    /// synthesized.
    #[serde(default)]
    pub license_basis: Option<String>,
}

/// What one apply wrote, for the wizard's summary line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub written: Vec<String>,
    /// Selections whose exact bytes were already at the destination.
    pub already_present: Vec<String>,
}

/// The bytes of one origin: a single file or a whole skill tree.
enum Bytes {
    File(Vec<u8>),
    Tree(Vec<(PathBuf, Vec<u8>)>),
}

impl Bytes {
    fn hash(&self) -> String {
        match self {
            Bytes::File(bytes) => crate::hash::hash_bytes(bytes),
            Bytes::Tree(files) => crate::hash::hash_files(files),
        }
    }
}

/// Every package the given scopes hold, grouped and deduplicated. Origins
/// whose bytes cannot be read right now are listed with an empty hash so
/// the wizard can show them; selecting one refuses at apply.
pub fn inventory(env: &Env, scopes: &[Scope]) -> Result<Vec<ImportCandidate>> {
    let unmanaged = unmanaged_paths(env, scopes);
    let mut candidates: BTreeMap<(ItemKind, String), Vec<CandidateOrigin>> = BTreeMap::new();
    for row in crate::library::provenance(env, scopes)? {
        let Some((group, bytes, location)) = origin_bytes(env, &row, &unmanaged) else {
            continue;
        };
        let hash = bytes.map(|bytes| bytes.hash()).unwrap_or_default();
        let origins = candidates.entry((row.kind, row.name.clone())).or_default();
        match origins
            .iter_mut()
            .find(|origin| origin.hash == hash && origin.group == group)
        {
            Some(origin) => {
                if !origin.locations.contains(&location) {
                    origin.locations.push(location);
                }
            }
            None => origins.push(CandidateOrigin {
                group,
                locations: vec![location],
                hash,
            }),
        }
    }
    Ok(candidates
        .into_iter()
        .map(|((kind, name), mut origins)| {
            origins.sort_by_key(|origin| match origin.group {
                CandidateGroup::Own => 0u8,
                CandidateGroup::Marketplace { .. } => 1,
                CandidateGroup::Unmanaged => 2,
            });
            ImportCandidate {
                kind,
                name_problem: crate::names::item_problem(&name),
                name,
                origins,
            }
        })
        .collect())
}

/// The observed on-disk path of every unmanaged installation — provenance
/// rows carry no path, so the scan is asked once and joined here.
fn unmanaged_paths(env: &Env, scopes: &[Scope]) -> BTreeMap<(Scope, ItemKind, String), PathBuf> {
    let Ok(settings) = crate::settings::load(env) else {
        return BTreeMap::new();
    };
    let scopes: Vec<Scope> = scopes.iter().map(Scope::canonical).collect();
    let observed = crate::scan::scan_scopes(env, &settings.harness_roots, &scopes);
    let mut paths = BTreeMap::new();
    for item in observed.items {
        if item.vendor.is_some() {
            continue;
        }
        paths
            .entry((item.scope, item.kind, item.name))
            .or_insert(item.path);
    }
    paths
}

/// Where one provenance row's bytes live, or `None` for rows import cannot
/// carry (vendor content never reaches provenance; config-entry kinds have
/// no file of their own to copy).
fn origin_bytes(
    env: &Env,
    row: &crate::library::ProvenanceRow,
    unmanaged: &BTreeMap<(Scope, ItemKind, String), PathBuf>,
) -> Option<(CandidateGroup, Option<Bytes>, String)> {
    match &row.origin {
        Origin::Own { .. } => {
            let root = crate::source::local_source_root(env, &row.scope);
            let (bytes, location) = catalog_bytes(&root, LOCAL_SOURCE_NAME, row)?;
            Some((CandidateGroup::Own, bytes, location))
        }
        Origin::Marketplace { source, repo } => {
            let manifest = scope_manifest(env, &row.scope);
            let group = CandidateGroup::Marketplace {
                source: source.clone(),
                repo: repo.clone(),
                license: None,
            };
            let resolved = match crate::source::resolve(env, &row.scope, source, &manifest) {
                Ok(crate::source::SourceState::Ready(resolved)) => resolved,
                // Unreachable provenance is listed, not guessed: the row
                // shows with no bytes and selecting it refuses.
                _ => return Some((group, None, format!("{repo} (not fetched)"))),
            };
            let sealed = SealedSource::open(&resolved.root).ok()?;
            let config = crate::source::source_config_for(&sealed, &resolved.provenance).ok()?;
            let license = config
                .marketplace
                .as_ref()
                .and_then(|meta| meta.license.clone());
            let group = CandidateGroup::Marketplace {
                source: source.clone(),
                repo: repo.clone(),
                license,
            };
            let path = crate::source::find_item(&sealed, &config, row.kind, &row.name)?;
            let bytes = read_bytes(&sealed, row.kind, &path)?;
            Some((
                group,
                Some(bytes),
                format!("{repo}:{}", rel(&sealed, &path)),
            ))
        }
        Origin::Unmanaged => {
            if !matches!(
                row.kind,
                ItemKind::Skill | ItemKind::Agent | ItemKind::Command
            ) {
                return None;
            }
            let path = unmanaged.get(&(row.scope.clone(), row.kind, row.name.clone()))?;
            let sealed = SealedSource::open(path.parent()?).ok()?;
            let bytes = read_bytes(&sealed, row.kind, path)?;
            Some((
                CandidateGroup::Unmanaged,
                Some(bytes),
                path.display().to_string(),
            ))
        }
    }
}

fn catalog_bytes(
    root: &Path,
    provenance: &str,
    row: &crate::library::ProvenanceRow,
) -> Option<(Option<Bytes>, String)> {
    let sealed = SealedSource::open(root).ok()?;
    let config = crate::source::source_config_for(&sealed, provenance).ok()?;
    let path = crate::source::find_item(&sealed, &config, row.kind, &row.name)?;
    let bytes = read_bytes(&sealed, row.kind, &path)?;
    let location = root.join(rel(&sealed, &path)).display().to_string();
    Some((Some(bytes), location))
}

fn read_bytes(sealed: &SealedSource, kind: ItemKind, path: &Path) -> Option<Bytes> {
    match kind {
        ItemKind::Skill => {
            let dir = match sealed.is_dir(path) {
                true => path.to_path_buf(),
                // A one-skill repo hands the SKILL.md itself.
                false => path.parent()?.to_path_buf(),
            };
            let files = sealed.collect_skill_tree(&dir).ok()?;
            Some(Bytes::Tree(files))
        }
        _ => Some(Bytes::File(sealed.read(path).ok()?)),
    }
}

fn rel(sealed: &SealedSource, path: &Path) -> String {
    path.strip_prefix(sealed.root())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn scope_manifest(env: &Env, scope: &Scope) -> Manifest {
    crate::manifest::load(&crate::manifest::manifest_path(env, scope))
        .ok()
        .and_then(|file| match file {
            ManifestFile::Current(manifest) => Some(*manifest),
            _ => None,
        })
        .unwrap_or_default()
}

/// The bytes behind one selection, re-read now so the hash the person saw
/// is revalidated against what is on disk at apply time.
fn selection_bytes(env: &Env, scopes: &[Scope], selection: &ImportSelection) -> Result<Bytes> {
    let unmanaged = unmanaged_paths(env, scopes);
    for row in crate::library::provenance(env, scopes)? {
        if row.kind != selection.kind || row.name != selection.name {
            continue;
        }
        if let Some((_, Some(bytes), _)) = origin_bytes(env, &row, &unmanaged)
            && bytes.hash() == selection.hash
        {
            return Ok(bytes);
        }
    }
    Err(CoreError::Authoring {
        message: format!(
            "the bytes of {} '{}' changed since the preview — re-open the import to re-preview",
            selection.kind.name(),
            selection.name
        ),
    })
}

mod apply;
pub use apply::apply;

#[cfg(test)]
mod tests;
