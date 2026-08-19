//! Every reason the hooks machinery refuses instead of proceeding. Each
//! guards an ownership invariant — kendex only ever removes or points at
//! what it provably wrote — and a refusal is the whole response: no
//! adoption, no repair, no partial mutation.

use std::collections::BTreeSet;

use crate::error::{CoreError, Result};
use crate::process::Hardened;

use super::{HOOKS, RECEIPT_FILE, Receipt, Repo, V1_SENTINEL, err};

/// Install-time refusals, all checked before any mutation is planned.
pub(super) fn check_install(repo: &Repo, receipt: Option<&Receipt>) -> Result<()> {
    let hooks_dir = repo.hooks_dir()?;
    if hooks_dir.is_symlink() {
        return Err(err(format!(
            "{} is a symlink — kendex refuses to adopt a directory it did not create; remove the link and rerun",
            hooks_dir.display()
        )));
    }
    // A directory holding nothing but our own entrypoints byte for byte is
    // provably ours even without its receipt; anything else in it is not.
    if hooks_dir.exists() && receipt.is_none() {
        let foreign = super::uninstall::foreign_entries(&hooks_dir)?;
        if !foreign.is_empty() {
            return Err(err(format!(
                "{} already exists and carries no kendex receipt (holding {}) — a pre-existing directory is a refusal, not an adoption; move it aside and rerun",
                hooks_dir.display(),
                foreign.join(", ")
            )));
        }
    }
    // A write through a symlink lands wherever the link points; a hook
    // slot that has become a link is not a file kendex wrote.
    for name in HOOKS.iter().chain(std::iter::once(&RECEIPT_FILE)) {
        let slot = hooks_dir.join(name);
        if slot.is_symlink() {
            return Err(err(format!(
                "{} is a symlink — kendex refuses to write through a link it did not create; remove it and rerun",
                slot.display()
            )));
        }
    }

    // The v1 shim is a decommission, not a chain: chained, it runs the
    // guards twice while the v1 skill is installed and fails closed on
    // every commit once it is gone. The entrypoints refuse it at commit
    // time for the same hooks, so install must refuse the same set.
    for hook in HOOKS {
        let v1_hook = repo.common_dir.join("hooks").join(hook);
        if let Some(text) = crate::fs::read_if_exists(&v1_hook)?
            && text.contains(V1_SENTINEL)
        {
            return Err(err(format!(
                "{} is v1's vstack-guards shim — remove it first (v1: install-git-hooks --uninstall, or delete the shim), then rerun",
                v1_hook.display()
            )));
        }
    }

    // `extensions.worktreeConfig` and `includeIf` mean each linked
    // worktree can see a different core.hooksPath; checking only the
    // installing worktree could hijack or miss another's hooks. The
    // effective value, with its origin, must be ours-or-absent in every
    // worktree git's own registry lists — an unreadable worktree is a
    // refusal. Both generation paths are ours even when the directory is
    // gone: refusing to re-arm a repo whose old-named directory was
    // deleted would be hijacking nothing but our own leftover value.
    let ours: BTreeSet<String> = repo
        .generation_dirs()
        .iter()
        .map(|dir| dir.display().to_string())
        .chain(receipt.map(|r| r.hooks_path.clone()))
        .collect();
    for worktree in repo.worktrees()? {
        let unreadable = |detail: String| {
            err(format!(
                "worktree {} could not be read while checking its effective core.hooksPath — an unreadable worktree is a refusal: {detail}",
                worktree.display()
            ))
        };
        let output = Hardened::git(
            &["config", "--show-origin", "--get", "core.hooksPath"],
            Some(&worktree),
        )
        .run()
        .map_err(|error| unreadable(error.to_string()))?;
        match output.status.code() {
            Some(1) => continue, // absent here — fine
            Some(0) => {
                let text = String::from_utf8_lossy(&output.stdout);
                let line = text.trim();
                let (origin, value) = line.split_once('\t').unwrap_or(("", line));
                if !ours.contains(value) {
                    return Err(err(format!(
                        "worktree {} already resolves core.hooksPath to '{value}' (from {origin}) — kendex refuses to hijack it; unset it there and rerun",
                        worktree.display()
                    )));
                }
            }
            _ => {
                return Err(unreadable(
                    String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                ));
            }
        }
    }
    Ok(())
}

/// Uninstall-time refusal: files kendex didn't write found in the owned
/// directory. Unsetting `core.hooksPath` around a surviving user hook
/// would silently disable it, so partial removal refuses instead.
pub(super) fn check_uninstall(repo: &Repo, receipt: &Receipt) -> Result<()> {
    let hooks_dir = repo.hooks_dir()?;
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
            "{} holds file(s) kendex did not write ({}) — removing around them would silently disable them the moment core.hooksPath is unset; move them into git's own hooks directory (or delete them) and rerun",
            hooks_dir.display(),
            foreign.join(", ")
        )));
    }
    Ok(())
}
