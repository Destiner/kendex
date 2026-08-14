//! Reading and writing a manifest file: what sits at a path, how a
//! mutation upgrades the schema as a side effect of writing at all, and
//! the one place a new scope gets its default source.

use std::path::Path;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::fs::{atomic_write, read_if_exists};
use crate::model::{HarnessId, Scope};

use super::{
    DEFAULT_SOURCE_NAME, DEFAULT_SOURCE_REPO, Finding, MANIFEST_SCHEMA, Manifest, SourceDecl,
    validate,
};

/// What sits at a manifest path. A schema-less file is a v1 manifest: v2
/// never mutates it — hard "migration required" error until the importer.
#[derive(Debug, Clone, PartialEq)]
pub enum ManifestFile {
    Absent,
    Legacy { raw: String },
    Current(Box<Manifest>),
}

pub fn manifest_path(env: &Env, scope: &Scope) -> std::path::PathBuf {
    match scope {
        Scope::Global => env.global_manifest_file(),
        Scope::Project { root } => Env::project_manifest_file(root),
    }
}

pub fn load(path: &Path) -> Result<ManifestFile> {
    let Some(text) = read_if_exists(path)? else {
        return Ok(ManifestFile::Absent);
    };
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| CoreError::TomlParse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
    if !table.contains_key("schema") {
        return Ok(ManifestFile::Legacy { raw: text });
    }
    if let Some(schema) = table.get("schema").and_then(toml::Value::as_integer)
        && schema > i64::from(MANIFEST_SCHEMA)
    {
        return Err(CoreError::SchemaTooNew {
            path: path.to_path_buf(),
            found: schema,
        });
    }
    let findings = validate(&table);
    if !findings.is_empty() {
        return Err(CoreError::ManifestInvalid {
            path: path.to_path_buf(),
            findings: findings.iter().map(Finding::to_string).collect(),
        });
    }
    let manifest: Manifest =
        toml::from_str(&text).map_err(|e: toml::de::Error| CoreError::TomlParse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
    Ok(ManifestFile::Current(Box::new(manifest)))
}

pub fn save(path: &Path, manifest: &Manifest) -> Result<()> {
    let text = toml::to_string_pretty(manifest).map_err(|e| CoreError::TomlParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    atomic_write(path, &text)
}

/// Load for mutation: a legacy file is a hard error, never a write target.
/// Whatever schema was read, a mutation writes the current one — every
/// write path upgrades as a side effect of writing at all.
pub fn load_for_mutation(path: &Path) -> Result<Option<Manifest>> {
    match load(path)? {
        ManifestFile::Absent => Ok(None),
        ManifestFile::Legacy { .. } => Err(CoreError::LegacyManifest {
            path: path.to_path_buf(),
        }),
        ManifestFile::Current(mut manifest) => {
            manifest.schema = MANIFEST_SCHEMA;
            Ok(Some(*manifest))
        }
    }
}

/// First manifest for a scope: the default source is seeded exactly once,
/// here — later reconciliation never re-adds it (its removal is durable).
pub fn seed(detected_harnesses: &[HarnessId]) -> Manifest {
    let mut manifest = Manifest {
        schema: MANIFEST_SCHEMA,
        ..Manifest::default()
    };
    manifest.sources.insert(
        DEFAULT_SOURCE_NAME.to_owned(),
        SourceDecl {
            repo: Some(DEFAULT_SOURCE_REPO.to_owned()),
            path: None,
            rev: None,
            enabled: true,
        },
    );
    manifest.install.harnesses = detected_harnesses.to_vec();
    manifest
}
