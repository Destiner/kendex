//! Which catalog a declaration reads from, and what it costs the pass when
//! that catalog cannot be read. Nothing here fails the scope: a source that
//! is switched off, not downloaded yet, gone from disk, or dressed up to
//! read outside itself costs the declarations that name it and nothing more.

use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::manifest::{ItemDecl, Manifest};
use crate::model::Scope;
use crate::source::{self, SourceConfig, SourceState, source_config};
use crate::source_read::SealedSource;

use super::desired::DesiredState;

/// The source root and provenance to build an item from, or `None` with the
/// note that says why this declaration produces nothing this pass.
pub(super) fn resolve_source(
    env: &Env,
    scope: &Scope,
    name: &str,
    decl: &ItemDecl,
    manifest: &Manifest,
    state: &mut DesiredState,
) -> Result<Option<(PathBuf, String)>> {
    let resolution = match state.sources.get(&decl.source) {
        Some(resolution) => resolution.clone(),
        None => {
            let resolution = source::resolve(env, scope, &decl.source, manifest)?;
            state
                .sources
                .insert(decl.source.clone(), resolution.clone());
            resolution
        }
    };
    let notes = &mut state.notes;
    match resolution {
        SourceState::Ready(ready) => Ok(Some((ready.root, ready.provenance))),
        // A disabled source deactivates its installations in place; they stay
        // declared and are not drift.
        SourceState::Disabled { .. } => {
            notes.push(format!(
                "{name}: source '{}' disabled — inactive",
                decl.source
            ));
            Ok(None)
        }
        SourceState::Pending { repo, .. } => {
            notes.push(format!(
                "{name}: source '{}' ({repo}) not fetched yet — skipped",
                decl.source
            ));
            Ok(None)
        }
        SourceState::Missing { path, .. } => {
            notes.push(format!(
                "{name}: source '{}' missing at {} — skipped",
                decl.source,
                path.display()
            ));
            Ok(None)
        }
    }
}

/// The catalog behind one declaration: its sealed root — every read goes
/// through one, so a hostile catalog cannot smuggle host files in — and its
/// layout tables. `None` with the note that says why this declaration
/// produces nothing this pass: a root that cannot be opened is skipped like
/// a missing one, and a registry or config dressed up to read outside the
/// catalog costs this declaration and nothing else.
pub(super) fn read_catalog(
    root: &Path,
    name: &str,
    source: &str,
    state: &mut DesiredState,
) -> Result<Option<(SealedSource, SourceConfig)>> {
    let sealed = match SealedSource::open(root) {
        Ok(sealed) => sealed,
        Err(problem) => {
            state.notes.push(format!(
                "{name}: source '{source}' unreadable ({problem}) — skipped"
            ));
            return Ok(None);
        }
    };
    match source_config(&sealed) {
        Ok(config) => Ok(Some((sealed, config))),
        Err(CoreError::SourceEscape { path, reason }) => {
            state.notes.push(format!(
                "{name}: unreadable — refused catalog read: {reason} ({})",
                path.display()
            ));
            Ok(None)
        }
        Err(other) => Err(other),
    }
}
