use std::path::PathBuf;

use thiserror::Error;

use crate::model::{HarnessId, ItemKind};

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("cannot locate the home directory on this system")]
    NoHomeDir,

    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path}: invalid TOML: {message}")]
    TomlParse { path: PathBuf, message: String },

    #[error("{path}: invalid JSON: {message}")]
    JsonParse { path: PathBuf, message: String },

    #[error("{path} is not a directory")]
    NotADirectory { path: PathBuf },

    #[error("project already registered: {path}")]
    ProjectAlreadyRegistered { path: PathBuf },

    #[error("project not registered: {path}")]
    ProjectNotRegistered { path: PathBuf },

    #[error("{path}: invalid manifest:\n{}", findings.join("\n"))]
    ManifestInvalid {
        path: PathBuf,
        findings: Vec<String>,
    },

    #[error(
        "{path} is a v1 manifest (no schema key) — migration required; v2 never modifies v1 files (the importer arrives with the release)"
    )]
    LegacyManifest { path: PathBuf },

    #[error("{path} is a v1 vstack lock — migration required; v2 never modifies v1 files")]
    LegacyLock { path: PathBuf },

    #[error("{path}: this lock file is damaged and could not be read — {message}")]
    LockCorrupt { path: PathBuf, message: String },

    #[error(
        "{path} was written by a newer vstack (format {found}) — update this app before touching it"
    )]
    SchemaTooNew { path: PathBuf, found: i64 },

    #[error("{path}: refused catalog read — {reason}")]
    SourceEscape { path: PathBuf, reason: String },

    #[error("'{name}' already installed from {existing} — refusing to rebind to {requested}")]
    SourceCollision {
        name: String,
        existing: String,
        requested: String,
    },

    #[error("{target} is a foreign symlink (→ {points_to}) — conflict, not a clobber target")]
    ForeignSymlink { target: PathBuf, points_to: PathBuf },

    #[error("scope is busy: another apply holds {lock}")]
    ScopeBusy { lock: PathBuf },

    #[error("source cache is busy: another download holds {lock}")]
    CacheBusy { lock: PathBuf },

    #[error(
        "{repo} is pinned to {pin}, which is not in the cache and could not be fetched: {reason}"
    )]
    PinUnavailable {
        repo: String,
        pin: String,
        reason: String,
    },

    #[error("plan is stale: {path} changed since the plan was computed — re-plan and retry")]
    PlanStale { path: PathBuf },

    #[error("source '{name}' has not been downloaded yet — refresh it first")]
    SourcePending { name: String },

    #[error("source '{name}' is disabled")]
    SourceDisabled { name: String },

    #[error("source '{name}' points at {path}, which does not exist")]
    SourceMissing { name: String, path: PathBuf },

    #[error("unknown source '{name}' — declare [sources.{name}] first")]
    UnknownSource { name: String },

    #[error("'{name}' not found in source '{source_name}'")]
    ItemNotInSource { name: String, source_name: String },

    #[error("no item from source '{source_name}' offers '{name}' as an optional dependency")]
    NoSuchOptional { name: String, source_name: String },

    #[error("source '{source_name}' offers no bundle called '{name}'")]
    NoSuchBundle { name: String, source_name: String },

    #[error("apply failed and was rolled back: {reason}")]
    RolledBack { reason: String },

    #[error("{path}: structured edit failed: {message}")]
    ConfigEdit { path: PathBuf, message: String },

    #[error("pi package {name}: {message}")]
    PiPackage { name: String, message: String },

    #[error("no {} named '{name}' found for {} in this scope", kind.name(), harness.name())]
    ItemNotFound {
        kind: ItemKind,
        name: String,
        harness: HarnessId,
    },

    #[error("{command} failed: {stderr}")]
    GitFailed { command: String, stderr: String },
}

impl CoreError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        CoreError::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
