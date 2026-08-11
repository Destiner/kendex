use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use super::journal;
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::hash::hash_tree;
use crate::lock::Lock;
use crate::manifest::Manifest;
use crate::model::Scope;

/// A precondition every mutation revalidates immediately before running —
/// plans bind to the observed state they were computed from (invariant 7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "pre", rename_all = "kebab-case")]
pub enum Pre {
    Absent,
    HashIs { hash: String },
    SymlinkTo { target: PathBuf },
    Any,
}

impl Pre {
    /// What a plan that rewrites `path` wholesale binds to: the bytes seen
    /// at plan time, or the absence seen at plan time.
    pub fn observed(path: &Path) -> Result<Pre> {
        match path.is_file() {
            true => Ok(Pre::HashIs {
                hash: hash_tree(path)?,
            }),
            false => Ok(Pre::Absent),
        }
    }

    fn check(&self, path: &Path) -> Result<()> {
        let ok = match self {
            Pre::Any => true,
            Pre::Absent => !path.exists() && !path.is_symlink(),
            Pre::HashIs { hash } => {
                !path.is_symlink()
                    && path.exists()
                    && hash_tree(path).map(|h| h == *hash).unwrap_or(false)
            }
            Pre::SymlinkTo { target } => {
                path.is_symlink() && fs::read_link(path).ok().as_deref() == Some(target)
            }
        };
        if ok {
            Ok(())
        } else {
            Err(CoreError::PlanStale {
                path: path.to_path_buf(),
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    WriteFile {
        path: PathBuf,
        bytes: Vec<u8>,
        pre: Pre,
    },
    /// Replace `root` wholesale with the given rendered tree.
    WriteTree {
        root: PathBuf,
        files: Vec<(PathBuf, Vec<u8>)>,
        pre: Pre,
    },
    Symlink {
        link: PathBuf,
        target: PathBuf,
        pre: Pre,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
        /// Checked against `to`: rename(2) replaces its destination
        /// silently, so a file that appeared since planning must abort.
        to_pre: Pre,
    },
    /// Removal never deletes: the artifact moves to the trash.
    Trash { path: PathBuf, pre: Pre },
    /// Apply an idempotent structured edit to a (possibly absent) config
    /// file — unrelated keys always survive.
    EditFile {
        path: PathBuf,
        edit: crate::configedit::ConfigEdit,
        pre: Pre,
    },
    /// Both records are written as whole plan-time snapshots, so `pre`
    /// keeps a stale plan from reverting a concurrent apply's work.
    WriteLock {
        path: PathBuf,
        lock: Box<Lock>,
        pre: Pre,
    },
    WriteManifest {
        path: PathBuf,
        manifest: Box<Manifest>,
        pre: Pre,
    },
}

impl Op {
    /// Every path this op mutates — journaled before execution.
    pub(super) fn touched(&self) -> Vec<PathBuf> {
        match self {
            Op::WriteFile { path, .. } => vec![path.clone()],
            Op::WriteTree { root, .. } => vec![root.clone()],
            Op::Symlink { link, .. } => vec![link.clone()],
            Op::Rename { from, to, .. } => vec![from.clone(), to.clone()],
            Op::Trash { path, .. } => vec![path.clone()],
            Op::EditFile { path, .. } => vec![path.clone()],
            Op::WriteLock { path, .. } => vec![path.clone()],
            Op::WriteManifest { path, .. } => vec![path.clone()],
        }
    }

    pub(super) fn run(&self, env: &Env) -> Result<()> {
        match self {
            Op::WriteFile { path, bytes, pre } => {
                pre.check(path)?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
                }
                fs::write(path, bytes).map_err(|e| CoreError::io(path, e))
            }
            Op::WriteTree { root, files, pre } => {
                pre.check(root)?;
                journal::remove_any(root)?;
                for (rel, bytes) in files {
                    let dest = root.join(rel);
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
                    }
                    fs::write(&dest, bytes).map_err(|e| CoreError::io(&dest, e))?;
                }
                Ok(())
            }
            Op::Symlink { link, target, pre } => {
                pre.check(link)?;
                if let Some(parent) = link.parent() {
                    fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
                }
                if link.is_symlink() {
                    fs::remove_file(link).map_err(|e| CoreError::io(link, e))?;
                }
                journal::make_symlink(target, link)
            }
            Op::Rename { from, to, to_pre } => {
                to_pre.check(to)?;
                fs::rename(from, to).map_err(|e| CoreError::io(from, e))
            }
            Op::Trash { path, pre } => {
                pre.check(path)?;
                let trash = env.trash_dir();
                fs::create_dir_all(&trash).map_err(|e| CoreError::io(&trash, e))?;
                let base = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "item".to_owned());
                let dest = unique_in(&trash, &base);
                if fs::rename(path, &dest).is_err() {
                    // Cross-device: copy then remove.
                    if path.is_dir() {
                        journal::copy_tree(path, &dest)?;
                    } else {
                        fs::copy(path, &dest).map_err(|e| CoreError::io(path, e))?;
                    }
                    journal::remove_any(path)?;
                }
                Ok(())
            }
            Op::EditFile { path, edit, pre } => {
                pre.check(path)?;
                let current = crate::fs::read_if_exists(path)?.unwrap_or_default();
                let updated = edit
                    .apply(&current)
                    .map_err(|message| CoreError::ConfigEdit {
                        path: path.clone(),
                        message,
                    })?;
                if updated != current {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
                    }
                    fs::write(path, updated).map_err(|e| CoreError::io(path, e))?;
                }
                Ok(())
            }
            Op::WriteLock { path, lock, pre } => {
                pre.check(path)?;
                crate::lock::save(path, lock)
            }
            Op::WriteManifest {
                path,
                manifest,
                pre,
            } => {
                pre.check(path)?;
                crate::manifest::save(path, manifest)
            }
        }
    }
}

fn unique_in(dir: &Path, base: &str) -> PathBuf {
    let stamp = crate::lock::timestamp().replace(':', "-");
    let mut candidate = dir.join(format!("{stamp}-{base}"));
    let mut counter = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{stamp}-{counter}-{base}"));
        counter += 1;
    }
    candidate
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedOp {
    pub description: String,
    pub op: Op,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub scope: Scope,
    pub ops: Vec<PlannedOp>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}
