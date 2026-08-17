//! The repository one guard invocation judges: its top level, and the index
//! git named for this commit. Every git read the guards make goes through
//! here — NUL-delimited raw bytes end to end, so a path the configuration
//! format cannot represent is a loud refusal, never a silently split row.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};
use crate::process::Hardened;

use super::guard_err;

/// One guard run's repository binding.
#[derive(Debug)]
pub struct GuardCtx {
    /// The working tree's top level.
    pub root: PathBuf,
    /// The index this commit is being built from — `GIT_INDEX_FILE` as the
    /// hook entrypoint captured it, canonicalized once. `None` means the
    /// repository's ordinary index.
    pub index_file: Option<PathBuf>,
}

impl GuardCtx {
    /// Bind to the repository containing `dir`, capturing the environment's
    /// `GIT_INDEX_FILE` before anything else can scrub or outrun it. The
    /// value is resolved against the current working directory — git hands
    /// hooks a relative path — and must exist: a named index that is not
    /// there is exit 2, never a fallback to the wrong index.
    pub fn bind(dir: &Path) -> Result<GuardCtx> {
        let output = Hardened::git(&["rev-parse", "--show-toplevel"], Some(dir)).run()?;
        if !output.status.success() {
            return Err(guard_err("guard", "not inside a git repository"));
        }
        let root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim().to_owned());
        let index_file = match std::env::var_os("GIT_INDEX_FILE") {
            None => None,
            Some(raw) => {
                let path = PathBuf::from(raw);
                let absolute = match path.is_absolute() {
                    true => path,
                    false => std::env::current_dir()
                        .map_err(|e| CoreError::io("current dir", e))?
                        .join(path),
                };
                Some(absolute.canonicalize().map_err(|_| {
                    guard_err(
                        "guard",
                        format!(
                            "GIT_INDEX_FILE names {}, which does not exist — refusing to judge a different index",
                            absolute.display()
                        ),
                    )
                })?)
            }
        };
        Ok(GuardCtx { root, index_file })
    }

    /// A git invocation in this repository, reading this commit's index.
    pub fn git(&self, args: &[&str]) -> Hardened {
        let hardened = Hardened::git(args, Some(&self.root));
        match &self.index_file {
            Some(index) => hardened.index_file(index),
            None => hardened,
        }
    }

    /// Run and demand success; stderr travels into the error.
    pub fn git_ok(&self, check: &str, args: &[&str]) -> Result<Vec<u8>> {
        let output = self.git(args).run()?;
        if !output.status.success() {
            return Err(guard_err(
                check,
                format!(
                    "git {} failed: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            ));
        }
        Ok(output.stdout)
    }

    /// Run where exit 1 is a measurement ("no matches"), anything above is
    /// a failed collection.
    pub fn git_grep(&self, check: &str, args: &[&str]) -> Result<Vec<u8>> {
        let output = self.git(args).run()?;
        match output.status.code() {
            Some(0) | Some(1) => Ok(output.stdout),
            _ => Err(guard_err(
                check,
                format!(
                    "git {} failed collecting matches: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            )),
        }
    }

    /// Refuse the index states no guard can judge. Unmerged entries have
    /// several truths at once; an intent-to-add record stages an empty
    /// stand-in for content that exists only on disk — judging either
    /// silently would wave through exactly the files in question. The
    /// intent-to-add probe reads the staged diff, where such an entry is
    /// the one addition with no destination blob.
    pub fn assert_judgeable(&self, check: &str) -> Result<()> {
        let unmerged = self.git_ok(check, &["ls-files", "-uz"])?;
        if !unmerged.is_empty() {
            return Err(guard_err(
                check,
                "the index holds unmerged entries — resolve the conflicts before committing",
            ));
        }
        // Porcelain v2 names an intent-to-add entry unambiguously: a
        // changed-entry record whose staged state is '.' and worktree
        // state 'A' — the shape ls-files masks behind an empty stand-in
        // blob no honest check should judge.
        let raw = self.git_ok(
            check,
            &[
                "status",
                "--porcelain=v2",
                "-z",
                "--no-renames",
                "--untracked-files=no",
            ],
        )?;
        for record in raw.split(|byte| *byte == 0) {
            let record = String::from_utf8_lossy(record);
            let mut fields = record.split(' ');
            if fields.next() != Some("1") {
                continue;
            }
            if fields.next() == Some(".A") {
                let path = record.rsplit(' ').next().unwrap_or("");
                return Err(guard_err(
                    check,
                    format!(
                        "intent-to-add entry for '{path}' has no staged content to judge — stage it or unstage it"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// One blob's size, from the object store — the bytes that enter
    /// history. A blob that cannot be read is exit 2: its size is
    /// unmeasurable, and refusing beats skipping it.
    pub fn blob_size(&self, check: &str, sha: &str, path_shown: &str) -> Result<u64> {
        if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(guard_err(
                check,
                format!("invalid blob id for '{path_shown}'"),
            ));
        }
        let out = self.git_ok(check, &["cat-file", "-s", sha]).map_err(|_| {
            guard_err(
                check,
                format!("cannot read blob {sha} for '{path_shown}' — its size is unmeasurable, refusing to skip it"),
            )
        })?;
        String::from_utf8_lossy(&out)
            .trim()
            .parse()
            .map_err(|_| guard_err(check, format!("unparseable blob size for '{path_shown}'")))
    }

    /// One tracked file's content as the commit would record it.
    pub fn index_content(&self, _check: &str, path: &str) -> Result<Option<Vec<u8>>> {
        let output = self.git(&["show", &format!(":{path}")]).run()?;
        match output.status.success() {
            true => Ok(Some(output.stdout)),
            false => Ok(None),
        }
    }

    /// Per-file match counts over index content — `git grep -c` with `-z`,
    /// one subprocess — for the pathspecs given (none = every tracked
    /// file). Binary blobs are skipped. A record is `path NUL count NL`,
    /// so a path may carry a newline and still parse; one that is not
    /// UTF-8 is refused: no report or baseline can name it.
    pub fn grep_counts(
        &self,
        check: &str,
        ere: &str,
        pathspecs: &[&str],
    ) -> Result<BTreeMap<String, u64>> {
        let mut args = vec!["grep", "--cached", "-cIzE", ere, "--"];
        args.extend_from_slice(pathspecs);
        let raw = self.git_grep(check, &args)?;
        let mut counts = BTreeMap::new();
        let mut rest = raw.as_slice();
        while !rest.is_empty() {
            let Some(nul) = rest.iter().position(|byte| *byte == 0) else {
                return Err(guard_err(check, "unparseable count record from git grep"));
            };
            let (path, after) = rest.split_at(nul);
            let after = &after[1..];
            let newline = after
                .iter()
                .position(|byte| *byte == b'\n')
                .unwrap_or(after.len());
            let (count, tail) = after.split_at(newline);
            rest = tail.get(1..).unwrap_or(&[]);
            let Ok(path) = std::str::from_utf8(path) else {
                return Err(guard_err(
                    check,
                    format!(
                        "tracked path is not valid UTF-8 and cannot be represented in reports or baselines: {:?}",
                        String::from_utf8_lossy(path)
                    ),
                ));
            };
            let count: u64 = std::str::from_utf8(count)
                .ok()
                .and_then(|text| text.trim().parse().ok())
                .ok_or_else(|| guard_err(check, format!("unparseable match count for '{path}'")))?;
            counts.insert(path.to_owned(), count);
        }
        Ok(counts)
    }
}
