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
