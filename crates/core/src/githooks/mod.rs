//! A hooks directory vstack owns — provably, not declaratively.
//!
//! Install writes `<git-common-dir>/vstack-hooks/` with two entrypoints
//! and a receipt recording the exact files written and the exact
//! `core.hooksPath` value set. Repair rewrites only receipt-listed files;
//! uninstall deletes only receipt-listed files, removes the directory only
//! if empty after, and unsets `core.hooksPath` only while its current
//! value still equals the receipt's — compare-and-swap on both sides.
//! vstack never edits a hook file it did not create: a pre-existing or
//! symlinked `vstack-hooks` directory is a refusal, and so are foreign
//! files found there at uninstall, because unsetting `core.hooksPath`
//! around a surviving user hook would silently disable it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::apply::{Op, PlannedOp, Pre};
use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::model::Scope;
use crate::process::Hardened;

mod entrypoints;
mod refusals;
pub use entrypoints::{COMMIT_MSG_ENTRYPOINT, PRE_COMMIT_ENTRYPOINT};

pub const HOOKS_DIR: &str = "vstack-hooks";
pub const RECEIPT_FILE: &str = "receipt.json";
/// v1's shim sentinel: a hook carrying it is decommissioned, never chained.
pub const V1_SENTINEL: &str = "# vstack-guards-hook";

fn err(message: impl Into<String>) -> CoreError {
    crate::guard::guard_err("hooks", message)
}

/// One repository, resolved: the worktree that invoked us and the common
/// dir every linked worktree shares. Both canonical, because leases and
/// the registry compare paths as text: one worktree reached through a
/// symlink must never read as two, or as none.
pub struct Repo {
    pub worktree: PathBuf,
    pub common_dir: PathBuf,
}

impl Repo {
    pub fn at(dir: &Path) -> Result<Repo> {
        let output = Hardened::git(
            &["rev-parse", "--show-toplevel", "--git-common-dir"],
            Some(dir),
        )
        .run()?;
        if !output.status.success() {
            return Err(err("not inside a git repository"));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines();
        let (Some(worktree), Some(common)) = (lines.next(), lines.next()) else {
            return Err(err(format!(
                "git named no working tree for {} — hooks install into a checkout, not a bare repository",
                dir.display()
            )));
        };
        let worktree = PathBuf::from(worktree);
        let worktree = worktree
            .canonicalize()
            .map_err(|e| CoreError::io(&worktree, e))?;
        let common = PathBuf::from(common);
        let common_dir = match common.is_absolute() {
            true => common,
            false => worktree.join(common),
        };
        let common_dir = common_dir
            .canonicalize()
            .map_err(|e| CoreError::io(&common_dir, e))?;
        Ok(Repo {
            worktree,
            common_dir,
        })
    }

    fn scope(&self) -> Scope {
        Scope::Project {
            root: self.worktree.clone(),
        }
    }

    pub fn hooks_dir(&self) -> PathBuf {
        self.common_dir.join(HOOKS_DIR)
    }

    fn receipt_path(&self) -> PathBuf {
        self.hooks_dir().join(RECEIPT_FILE)
    }

    fn config_file(&self) -> PathBuf {
        self.common_dir.join("config")
    }

    /// Every worktree git's own registry lists, canonicalized where it
    /// still exists.
    pub fn worktrees(&self) -> Result<Vec<PathBuf>> {
        let output =
            Hardened::git(&["worktree", "list", "--porcelain"], Some(&self.worktree)).run()?;
        if !output.status.success() {
            return Err(err("git worktree list failed"));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .map(|path| {
                let path = PathBuf::from(path);
                path.canonicalize().unwrap_or(path)
            })
            .collect())
    }
}

/// The recorded proof of ownership: exactly what was written, exactly what
/// was set, and which worktrees enabled the install (the leases uninstall
/// counts down).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Receipt {
    pub schema: u32,
    /// The exact `core.hooksPath` value set.
    pub hooks_path: String,
    /// Files written into the hooks directory, by name.
    pub files: Vec<String>,
    /// Worktree roots that enabled the install. The install stays armed
    /// while any lease survives.
    pub leases: BTreeSet<String>,
}

pub fn load_receipt(repo: &Repo) -> Result<Option<Receipt>> {
    let Some(text) = crate::fs::read_if_exists(&repo.receipt_path())? else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| err(format!("unreadable hooks receipt: {e}")))
}

#[derive(Debug)]
pub struct HooksReport {
    pub lines: Vec<String>,
}

/// Install (or repair) the owned hooks directory for the repository at
/// `dir`, recording this worktree's lease. The plan — refusals included
/// (see [`refusals`]) — is built under the common lock, so the directory
/// shape the refusals observe is the shape the writes land in.
pub fn install(env: &Env, dir: &Path) -> Result<HooksReport> {
    let repo = Repo::at(dir)?;
    let (_, hooks_path) =
        crate::apply::execute_common(env, &repo.scope(), &repo.common_dir, || plan_install(&repo))?;
    Ok(HooksReport {
        lines: vec![format!(
            "commit checks installed: core.hooksPath -> {hooks_path} (covers every linked worktree)"
        )],
    })
}

fn plan_install(repo: &Repo) -> Result<(Vec<PlannedOp>, String)> {
    let receipt = load_receipt(repo)?;
    refusals::check_install(repo, receipt.as_ref())?;

    let hooks_dir = repo.hooks_dir();
    let hooks_path = hooks_dir.display().to_string();
    let mut leases = receipt.map(|r| r.leases).unwrap_or_default();
    leases.insert(repo.worktree.display().to_string());
    let updated = Receipt {
        schema: 1,
        hooks_path: hooks_path.clone(),
        files: vec![
            "pre-commit".to_owned(),
            "commit-msg".to_owned(),
            RECEIPT_FILE.to_owned(),
        ],
        leases,
    };

    let mut ops = Vec::new();
    for (name, bytes) in [
        ("pre-commit", PRE_COMMIT_ENTRYPOINT),
        ("commit-msg", COMMIT_MSG_ENTRYPOINT),
    ] {
        let path = hooks_dir.join(name);
        ops.push(PlannedOp {
            description: format!("write the {name} entrypoint"),
            op: Op::WriteExecutable {
                pre: Pre::observed(&path)?,
                path,
                bytes: bytes.as_bytes().to_vec(),
            },
        });
    }
    let receipt_path = repo.receipt_path();
    ops.push(PlannedOp {
        description: "record the ownership receipt".into(),
        op: Op::WriteFile {
            pre: Pre::observed(&receipt_path)?,
            path: receipt_path,
            bytes: render_receipt(&updated)?,
        },
    });
    let current = crate::apply::read_git_config(&repo.config_file(), "core.hooksPath")?;
    if current.as_deref() != Some(hooks_path.as_str()) {
        ops.push(PlannedOp {
            description: "point core.hooksPath at the owned directory".into(),
            op: Op::GitConfigSwap {
                file: repo.config_file(),
                key: "core.hooksPath".into(),
                expected: current,
                value: Some(hooks_path.clone()),
            },
        });
    }
    Ok((ops, hooks_path))
}

/// Release this worktree's lease; disarm only when the last one goes.
/// Dead worktrees — leases git's registry no longer lists — are reaped in
/// passing. Removal is receipt-scoped with compare-and-swap on both
/// sides: a user-changed hooksPath survives, and a user-added file turns
/// uninstall into a refusal, never a half-removal.
pub fn uninstall(env: &Env, dir: &Path) -> Result<HooksReport> {
    let repo = Repo::at(dir)?;
    let (_, mut lines) =
        crate::apply::execute_common(env, &repo.scope(), &repo.common_dir, || {
            plan_uninstall(&repo)
        })?;
    // What is effective after decides what the user sees next.
    let effective = effective_hooks_path(&repo.worktree)?;
    lines.push(match effective {
        Some(path) => format!("effective core.hooksPath is now {path}"),
        None => "no core.hooksPath is in effect; git's own hooks directory applies".into(),
    });
    Ok(HooksReport { lines })
}

fn plan_uninstall(repo: &Repo) -> Result<(Vec<PlannedOp>, Vec<String>)> {
    let Some(receipt) = load_receipt(repo)? else {
        return plan_orphan_cleanup(repo);
    };
    let registry: BTreeSet<String> = repo
        .worktrees()?
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    let this = repo.worktree.display().to_string();
    let mut leases = receipt.leases.clone();
    let reaped: Vec<String> = leases
        .iter()
        .filter(|lease| !registry.contains(*lease))
        .cloned()
        .collect();
    for dead in &reaped {
        leases.remove(dead);
    }
    leases.remove(&this);

    let mut lines: Vec<String> = reaped
        .iter()
        .map(|dead| format!("reaped a lease for a worktree git no longer lists: {dead}"))
        .collect();

    if !leases.is_empty() {
        let updated = Receipt {
            leases: leases.clone(),
            ..receipt
        };
        let receipt_path = repo.receipt_path();
        let ops = vec![PlannedOp {
            description: "release this worktree's lease".into(),
            op: Op::WriteFile {
                pre: Pre::observed(&receipt_path)?,
                path: receipt_path,
                bytes: render_receipt(&updated)?,
            },
        }];
        lines.push(format!(
            "lease released — the commit checks stay armed for {} other worktree(s)",
            leases.len()
        ));
        return Ok((ops, lines));
    }

    refusals::check_uninstall(repo, &receipt)?;
    let hooks_dir = repo.hooks_dir();
    let mut ops = vec![PlannedOp {
        description: "remove the owned hooks directory".into(),
        op: Op::Trash {
            pre: Pre::HashIs {
                hash: crate::hash::hash_tree(&hooks_dir)?,
            },
            path: hooks_dir,
        },
    }];
    let current = crate::apply::read_git_config(&repo.config_file(), "core.hooksPath")?;
    if current.as_deref() == Some(receipt.hooks_path.as_str()) {
        ops.push(PlannedOp {
            description: "unset core.hooksPath".into(),
            op: Op::GitConfigSwap {
                file: repo.config_file(),
                key: "core.hooksPath".into(),
                expected: current,
                value: None,
            },
        });
        lines.push("commit checks removed; core.hooksPath unset".into());
    } else {
        lines.push(format!(
            "commit checks removed; core.hooksPath was changed by hand (now {:?}) and was left alone",
            current
        ));
    }
    Ok((ops, lines))
}

/// No receipt, but `core.hooksPath` still names the owned directory and
/// that directory is gone: someone removed the files by hand and left git
/// resolving hooks from nowhere — which silently runs no hook at all,
/// including the user's own. The config value is provably vstack's, so
/// taking it back is still receipt-scoped removal.
fn plan_orphan_cleanup(repo: &Repo) -> Result<(Vec<PlannedOp>, Vec<String>)> {
    let hooks_dir = repo.hooks_dir();
    let current = crate::apply::read_git_config(&repo.config_file(), "core.hooksPath")?;
    if hooks_dir.exists() || current.as_deref() != Some(hooks_dir.display().to_string().as_str()) {
        return Ok((
            Vec::new(),
            vec!["no vstack hooks are installed in this repository".into()],
        ));
    }
    let ops = vec![PlannedOp {
        description: "unset the core.hooksPath left behind by a hand-deleted hooks directory"
            .into(),
        op: Op::GitConfigSwap {
            file: repo.config_file(),
            key: "core.hooksPath".into(),
            expected: current,
            value: None,
        },
    }];
    Ok((
        ops,
        vec![format!(
            "the owned hooks directory {} was already gone; core.hooksPath still pointed at it and was unset",
            hooks_dir.display()
        )],
    ))
}

/// The `core.hooksPath` a worktree actually resolves, git's own answer.
pub fn effective_hooks_path(worktree: &Path) -> Result<Option<String>> {
    let output = Hardened::git(&["config", "--get", "core.hooksPath"], Some(worktree)).run()?;
    match output.status.code() {
        Some(0) => Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )),
        Some(1) => Ok(None),
        _ => Err(err(format!(
            "cannot read the effective core.hooksPath in {}",
            worktree.display()
        ))),
    }
}

fn render_receipt(receipt: &Receipt) -> Result<Vec<u8>> {
    let mut text = serde_json::to_string_pretty(receipt)
        .map_err(|e| err(format!("unrenderable receipt: {e}")))?;
    text.push('\n');
    Ok(text.into_bytes())
}
