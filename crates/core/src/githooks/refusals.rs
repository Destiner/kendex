//! Every reason the hooks machinery refuses instead of proceeding. The
//! goal was right in v1 and the machinery was the bug farm; here the
//! invariants that machinery defended are checks that say no.

use std::collections::BTreeSet;

use crate::error::{CoreError, Result};
use crate::process::Hardened;

use super::{Receipt, Repo, V1_SENTINEL};

fn err(message: impl Into<String>) -> CoreError {
    CoreError::Guard {
        check: "hooks".to_owned(),
        message: message.into(),
    }
}

/// Install-time refusals, all checked before any mutation is planned.
pub(super) fn check_install(repo: &Repo, receipt: Option<&Receipt>) -> Result<()> {
    let hooks_dir = repo.hooks_dir();
    if hooks_dir.is_symlink() {
        return Err(err(format!(
            "{} is a symlink — vstack refuses to adopt a directory it did not create; remove the link and rerun",
            hooks_dir.display()
        )));
    }
    if hooks_dir.exists() && receipt.is_none() {
        return Err(err(format!(
            "{} already exists and carries no vstack receipt — a pre-existing directory is a refusal, not an adoption; move it aside and rerun",
            hooks_dir.display()
        )));
    }

    // The v1 shim is a decommission, not a chain: chaining would run the
    // guards twice today and fail closed forever once the v1 skill is gone.
    let v1_hook = repo.common_dir.join("hooks").join("pre-commit");
    if let Some(text) = crate::fs::read_if_exists(&v1_hook)?
        && text.contains(V1_SENTINEL)
    {
        return Err(err(format!(
            "{} is v1's vstack-guards shim — remove it first (v1: install-git-hooks --uninstall, or delete the shim), then rerun",
            v1_hook.display()
        )));
    }

    // `extensions.worktreeConfig` and `includeIf` mean each linked
    // worktree can see a different core.hooksPath; checking only the
    // installing worktree could hijack or miss another's hooks. The
    // effective value, with its origin, must be ours-or-absent in every
    // worktree git's own registry lists — an unreadable worktree is a
    // refusal.
    let ours: BTreeSet<String> = [
        hooks_dir.display().to_string(),
        receipt.map(|r| r.hooks_path.clone()).unwrap_or_default(),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect();
    for worktree in repo.worktrees()? {
        let output = Hardened::git(
            &["config", "--show-origin", "--get", "core.hooksPath"],
            Some(&worktree),
        )
        .run()?;
        match output.status.code() {
            Some(1) => continue, // absent here — fine
            Some(0) => {
                let text = String::from_utf8_lossy(&output.stdout);
                let line = text.trim();
                let (origin, value) = line.split_once('\t').unwrap_or(("", line));
                if !ours.contains(value) {
                    return Err(err(format!(
                        "worktree {} already resolves core.hooksPath to '{value}' (from {origin}) — vstack refuses to hijack it; unset it there and rerun",
                        worktree.display()
                    )));
                }
            }
            _ => {
                return Err(err(format!(
                    "worktree {} could not be read while checking its effective core.hooksPath — an unreadable worktree is a refusal: {}",
                    worktree.display(),
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
        }
    }
    Ok(())
}

/// Uninstall-time refusal: files vstack didn't write found in the owned
/// directory. Unsetting `core.hooksPath` around a surviving user hook
/// would silently disable it, so partial removal refuses instead.
pub(super) fn check_uninstall(repo: &Repo, receipt: &Receipt) -> Result<()> {
    let hooks_dir = repo.hooks_dir();
    if !hooks_dir.exists() {
        return Ok(());
    }
    let recorded: BTreeSet<&str> = receipt.files.iter().map(String::as_str).collect();
    let entries = std::fs::read_dir(&hooks_dir).map_err(|e| CoreError::io(&hooks_dir, e))?;
    let mut foreign = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| CoreError::io(&hooks_dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !recorded.contains(name.as_str()) {
            foreign.push(name);
        }
    }
    if !foreign.is_empty() {
        return Err(err(format!(
            "{} holds file(s) vstack did not write ({}) — removing around them would silently disable them the moment core.hooksPath is unset; move them into git's own hooks directory (or delete them) and rerun",
            hooks_dir.display(),
            foreign.join(", ")
        )));
    }
    Ok(())
}
