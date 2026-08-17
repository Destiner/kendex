//! byte-ceiling — newly added tracked files over the byte ceiling fail
//! (default 200 KB). Sizes are object sizes (the added blob), the bytes
//! that enter history. Renames are detected, so moving an existing large
//! file is not an addition; a copy is one. Lockfiles are exempt built-in.

use crate::error::Result;

use super::settings::{Policy, config_path};
use super::{GuardCtx, Outcome, guard_err, patterns};

const CHECK: &str = "byte-ceiling";

/// Generated whole-file by package managers: size is not a design signal.
const LOCKFILE_BASENAMES: [&str; 17] = [
    "Cargo.lock",
    "package-lock.json",
    "npm-shrinkwrap.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "bun.lock",
    "bun.lockb",
    "flake.lock",
    "poetry.lock",
    "uv.lock",
    "Pipfile.lock",
    "Gemfile.lock",
    "composer.lock",
    "go.sum",
    "gradle.lockfile",
    "packages.lock.json",
    "Package.resolved",
];

fn is_lockfile(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    LOCKFILE_BASENAMES.contains(&base)
}

/// One staged addition: the blob and the path it lands at.
struct Added {
    sha: String,
    path: String,
}

/// `git diff --cached --raw -z --diff-filter=A` records alternate
/// "meta NUL path NUL"; meta is `:srcmode dstmode srcsha dstsha status`.
/// Only status A survives the filter, so each record carries one path.
fn staged_additions(ctx: &GuardCtx) -> Result<Vec<Added>> {
    let raw = ctx.git_ok(
        CHECK,
        &[
            "-c",
            "diff.renames=true",
            "diff",
            "--cached",
            "--raw",
            "--no-abbrev",
            "-z",
            "--diff-filter=A",
        ],
    )?;
    let mut added = Vec::new();
    let mut records = raw.split(|byte| *byte == 0);
    while let (Some(meta), Some(path)) = (records.next(), records.next()) {
        if meta.is_empty() {
            break;
        }
        let meta =
            std::str::from_utf8(meta).map_err(|_| guard_err(CHECK, "unparseable diff record"))?;
        let mut fields = meta.trim_start_matches(':').split(' ');
        let (_src, dstmode, _srcsha, dstsha) = (
            fields.next(),
            fields.next().unwrap_or(""),
            fields.next(),
            fields.next().unwrap_or(""),
        );
        // Symlinks and submodule gitlinks are not sized content.
        if dstmode == "120000" || dstmode == "160000" {
            continue;
        }
        let path = std::str::from_utf8(path).map_err(|_| {
            guard_err(
                CHECK,
                format!(
                    "added file has a non-UTF-8 path the report cannot carry: {:?}",
                    String::from_utf8_lossy(path)
                ),
            )
        })?;
        if dstsha.chars().all(|c| c == '0') {
            return Err(guard_err(
                CHECK,
                format!(
                    "added file '{path}' has no destination blob in the diff record — cannot measure it"
                ),
            ));
        }
        added.push(Added {
            sha: dstsha.to_owned(),
            path: path.to_owned(),
        });
    }
    Ok(added)
}

pub fn run(ctx: &GuardCtx, policy: &Policy) -> Result<Outcome> {
    let ceiling_kb = policy.positive_int(CHECK, "ceiling-kb", 200)?;
    let ceiling_bytes = ceiling_kb * 1024;
    let excludes_path = config_path(
        CHECK,
        &policy.string(CHECK, "excludes", "tools/byte-ceiling-excludes")?,
    )?;
    let excludes = patterns::load_excludes(ctx, CHECK, &excludes_path)?;

    let mut out = Outcome::default();
    let mut checked = 0usize;
    for added in staged_additions(ctx)? {
        if is_lockfile(&added.path) || excludes.is_excluded(&added.path) {
            continue;
        }
        let size = ctx.blob_size(CHECK, &added.sha, &added.path)?;
        checked += 1;
        if size > ceiling_bytes {
            let kb = size.div_ceil(1024);
            out.violation(
                format!(
                    "byte-ceiling FAIL oversized file: {} — {size} bytes (~{kb} KB) > ceiling {ceiling_kb} KB",
                    added.path
                ),
                &format!(
                    "keep big artifacts out of the repo (asset store, Git LFS, build-time generation); a file that genuinely belongs gets a row in {excludes_path} with its reason"
                ),
            );
        }
    }
    match out.violations {
        0 => out.say(format!(
            "byte-ceiling: OK — {checked} staged addition(s) checked, ceiling {ceiling_kb} KB"
        )),
        n => out.say(format!(
            "byte-ceiling: {n} violation(s) — ceiling {ceiling_kb} KB, {checked} staged addition(s) checked"
        )),
    }
    Ok(out)
}
