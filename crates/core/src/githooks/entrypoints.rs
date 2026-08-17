//! The two entrypoints the owned hooks directory carries. Their call
//! surface — `vstack guard run <hook>` — is a stable contract, so binary
//! upgrades never strand installed hooks. On guard pass each execs the
//! hook git itself would have run absent our `core.hooksPath` — resolved
//! via the common dir, since a linked worktree's `.git` is a file and the
//! literal path misses — and that hook's exit status decides. The one hook
//! never chained is v1's shim: install refuses it, and if it reappears
//! afterwards the chain would run the guards twice today and fail closed
//! forever once the v1 skill is gone, so the entrypoint refuses it too.

/// A missing binary fails closed: the message names the one-commit bypass
/// and the two-step manual removal, and nothing else. No vendored runner —
/// copies drift, which is the disease this machinery treats.
pub const PRE_COMMIT_ENTRYPOINT: &str = r#"#!/bin/sh
# vstack-hooks pre-commit — written by vstack; `vstack guard uninstall` removes it.
if ! command -v vstack >/dev/null 2>&1; then
  echo "vstack: this repository's commit checks need the vstack binary, which is not on PATH." >&2
  echo "  bypass this one commit:  git commit --no-verify" >&2
  echo "  remove the checks:       1) git config --unset core.hooksPath" >&2
  echo "                           2) delete the vstack-hooks directory inside the .git directory" >&2
  exit 1
fi
vstack guard run pre-commit || exit $?
next="$(git rev-parse --git-common-dir)/hooks/pre-commit"
if [ -x "$next" ]; then
  if grep -qF '# vstack-guards-hook' "$next" 2>/dev/null; then
    echo "vstack: $next is v1's vstack-guards shim, which vstack does not chain — remove it (v1: install-git-hooks --uninstall) and rerun vstack guard install" >&2
    exit 1
  fi
  exec "$next" "$@"
fi
exit 0
"#;

pub const COMMIT_MSG_ENTRYPOINT: &str = r#"#!/bin/sh
# vstack-hooks commit-msg — written by vstack; `vstack guard uninstall` removes it.
if ! command -v vstack >/dev/null 2>&1; then
  echo "vstack: this repository's commit checks need the vstack binary, which is not on PATH." >&2
  echo "  bypass this one commit:  git commit --no-verify" >&2
  echo "  remove the checks:       1) git config --unset core.hooksPath" >&2
  echo "                           2) delete the vstack-hooks directory inside the .git directory" >&2
  exit 1
fi
vstack guard run commit-msg "$1" || exit $?
next="$(git rev-parse --git-common-dir)/hooks/commit-msg"
if [ -x "$next" ]; then
  if grep -qF '# vstack-guards-hook' "$next" 2>/dev/null; then
    echo "vstack: $next is v1's vstack-guards shim, which vstack does not chain — remove it (v1: install-git-hooks --uninstall) and rerun vstack guard install" >&2
    exit 1
  fi
  exec "$next" "$@"
fi
exit 0
"#;
