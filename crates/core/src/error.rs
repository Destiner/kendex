use std::path::PathBuf;

use thiserror::Error;

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

    #[error("plan is stale: {path} changed since the plan was computed — re-plan and retry")]
    PlanStale { path: PathBuf },

    #[error("source '{name}' is not available locally yet (remote resolution arrives in Phase 5)")]
    SourcePending { name: String },

    #[error("source '{name}' is disabled")]
    SourceDisabled { name: String },

    #[error("source '{name}' points at {path}, which does not exist")]
    SourceMissing { name: String, path: PathBuf },

    #[error("unknown source '{name}' — declare [sources.{name}] first")]
    UnknownSource { name: String },

    #[error("'{name}' not found in source '{source_name}'")]
    ItemNotInSource { name: String, source_name: String },

    #[error("apply failed and was rolled back: {reason}")]
    RolledBack { reason: String },
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
