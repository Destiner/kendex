//! What a structured reader hands back for one entry it found in a file.

use std::path::PathBuf;

/// One parsed entry from a structured surface, before it becomes an
/// `ObservedItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    pub name: String,
    pub enabled: Option<bool>,
    pub description: Option<String>,
    /// Where this entry's own files live, when the reader knows and that is
    /// somewhere other than the file it was read from. A plugin cache lists
    /// every plugin in one place but each one has a directory of its own,
    /// and scoring a plugin against its neighbours' files is not scoring
    /// that plugin. `None` for entries that really do only exist as a line
    /// in a config file.
    pub source_path: Option<PathBuf>,
}
