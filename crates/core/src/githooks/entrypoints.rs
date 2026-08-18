//! The entrypoints the owned hooks directory carries. Their call surface —
//! `kendex guard run <hook>` — is a stable contract, so binary upgrades
//! never strand installed hooks. On guard pass each execs the hook git
//! itself would have run absent our `core.hooksPath` — resolved via the
//! common dir, since a linked worktree's `.git` is a file and the literal
//! path misses — and that hook's exit status decides. The one hook never
//! chained is v1's shim: install refuses it, and the entrypoint refuses it
//! too, because a chained shim runs the guards twice while the v1 skill
//! is installed and fails closed on every commit once it is gone.

use super::V1_SENTINEL;

/// The hooks the owned directory carries, in the order the receipt lists
/// them. Each git hook name is also the lane `kendex guard run` takes.
pub const HOOKS: [&str; 2] = ["pre-commit", "commit-msg"];

/// The entrypoint script for one hook. A missing binary fails closed: the
/// message names the one-commit bypass and the two-step manual removal,
/// and nothing else. No vendored runner — copies drift, which is the
/// disease this machinery treats. `commit-msg` forwards the message path
/// git hands it; `pre-commit` takes no argument.
pub fn entrypoint(hook: &str) -> String {
    let lane_args = lane_args(hook);
    format!(
        r#"#!/bin/sh
# kendex-hooks {hook} — written by kendex; `kendex guard uninstall` removes it.
if ! command -v kendex >/dev/null 2>&1; then
  echo "kendex: this repository's commit checks need the kendex binary, which is not on PATH." >&2
  echo "  bypass this one commit:  git commit --no-verify" >&2
  echo "  remove the checks:       1) git config --unset core.hooksPath" >&2
  echo "                           2) delete the kendex-hooks directory inside the .git directory" >&2
  exit 1
fi
kendex guard run {hook}{lane_args} || exit $?
next="$(git rev-parse --git-common-dir)/hooks/{hook}"
if [ -x "$next" ]; then
  if grep -qF '{V1_SENTINEL}' "$next" 2>/dev/null; then
    echo "kendex: $next is v1's vstack-guards shim, which kendex does not chain — remove it (v1: install-git-hooks --uninstall) and rerun kendex guard install" >&2
    exit 1
  fi
  exec "$next" "$@"
fi
exit 0
"#
    )
}

/// The exact bytes the vstack-named binary wrote into consuming repos.
/// Ownership by content must keep recognizing them for the alias cycle:
/// a directory holding nothing but these, byte for byte, is ours —
/// repaired by install, taken back by uninstall.
pub fn old_entrypoint(hook: &str) -> String {
    let lane_args = lane_args(hook);
    format!(
        r#"#!/bin/sh
# vstack-hooks {hook} — written by vstack; `vstack guard uninstall` removes it.
if ! command -v vstack >/dev/null 2>&1; then
  echo "vstack: this repository's commit checks need the vstack binary, which is not on PATH." >&2
  echo "  bypass this one commit:  git commit --no-verify" >&2
  echo "  remove the checks:       1) git config --unset core.hooksPath" >&2
  echo "                           2) delete the vstack-hooks directory inside the .git directory" >&2
  exit 1
fi
vstack guard run {hook}{lane_args} || exit $?
next="$(git rev-parse --git-common-dir)/hooks/{hook}"
if [ -x "$next" ]; then
  if grep -qF '{V1_SENTINEL}' "$next" 2>/dev/null; then
    echo "vstack: $next is v1's vstack-guards shim, which vstack does not chain — remove it (v1: install-git-hooks --uninstall) and rerun vstack guard install" >&2
    exit 1
  fi
  exec "$next" "$@"
fi
exit 0
"#
    )
}

fn lane_args(hook: &str) -> &'static str {
    match hook {
        "commit-msg" => " \"$1\"",
        _ => "",
    }
}
