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

    #[error(
        "both {new} and {old} exist — {old} was renamed to {new}, and both are here; keep the contents you mean, delete the other, and run again"
    )]
    BothGenerations { new: PathBuf, old: PathBuf },

    #[error("{path}: this lock file is damaged and could not be read — {message}")]
    LockCorrupt { path: PathBuf, message: String },

    #[error(
        "{path} was written by a newer kendex (format {found}) — update this app before touching it"
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

    // Said as what a person would see if they looked: the name they clicked
    // is a shortcut somebody else set up, and the files are somewhere else.
    // "Foreign symlink, not a clobber target" is the same fact in words that
    // only mean anything to whoever wrote the check.
    #[error(
        "{target} is a shortcut to {points_to}, not a folder of its own.          kendex only takes over files it can move, and moving this would          break whatever set the shortcut up."
    )]
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

    #[error(
        "source '{source_name}' is not a repository — only items from a repo source have revisions; remove the item's rev"
    )]
    ItemRevUnsupported { source_name: String },

    #[error(
        "no {} named '{name}' is declared in this scope — only declared items can be held at a version",
        kind.name()
    )]
    NotDeclared { kind: ItemKind, name: String },

    #[error("'{name}' does not exist in {repo} at {}", &commit[..commit.len().min(7)])]
    ItemMissingAtRev {
        name: String,
        repo: String,
        commit: String,
    },

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

    #[error("no accepted findings recorded under '{key}' in this scope")]
    OverrideNotFound { key: String },

    #[error("no dismissed finding {fingerprint} recorded under '{key}' in this scope")]
    DismissalNotFound { key: String, fingerprint: String },

    #[error(
        "the dismissal of {fingerprint} on '{key}' was replaced by a newer one — nothing was changed"
    )]
    DecisionReplaced { key: String, fingerprint: String },

    #[error(
        "'{token}' is not a decision token — expected kind:name:harness#fingerprint@hash, as printed beside the finding"
    )]
    DecisionToken { token: String },

    #[error("'{key}' does not name an installation — expected kind:name:harness")]
    DecisionKey { key: String },

    #[error(
        "'{token}' no longer names what is installed: {why} — nothing was changed; read the current findings and decide again"
    )]
    DecisionStale { token: String, why: String },

    #[error("no {} named '{name}' found for {} in this scope", kind.name(), harness.name())]
    ItemNotFound {
        kind: ItemKind,
        name: String,
        harness: HarnessId,
    },

    #[error("no {} named '{name}' found — not declared and nothing installed under that name", kind.name())]
    PackageNotFound { kind: ItemKind, name: String },

    #[error("{command} failed: {stderr}")]
    GitFailed { command: String, stderr: String },

    /// A guard's configuration is wrong or a measurement could not be
    /// taken — the loud exit-2 state, never a silent pass.
    #[error("{check}: {message}")]
    Guard { check: String, message: String },
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
