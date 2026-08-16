use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::metadata;
use crate::model::FileState;

pub struct FoundFile {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
    /// What the item says it is for, and anything it wrote where a tag
    /// belonged that is not one.
    pub meta: metadata::Metadata,
    pub modified_at: Option<u32>,
}

/// Best-effort: a stat that fails (permissions, a link race, a clock before
/// 1970 or past 2106) reports unavailable rather than failing the whole
/// scan over one timestamp.
pub fn mtime_unix(path: &Path) -> Option<u32> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u32::try_from(d.as_secs()).ok())
}

pub fn state_of(path: &Path) -> FileState {
    if path.is_symlink() {
        // Resolved, not as written. Two tools that link to one shared folder
        // spell the link differently (`../../.agents/...` from one place,
        // `../../../.agents/...` from another) — compared as written they
        // look like two unrelated items, and the app would offer to take
        // over each of them separately. Resolved, they are visibly one
        // thing in two places. A broken link has nothing to resolve, so it
        // keeps the text it was given, which is what names the problem.
        let target = fs::canonicalize(path)
            .or_else(|_| fs::read_link(path))
            .unwrap_or_default();
        FileState::Symlink {
            broken: !path.exists(),
            target,
        }
    } else if path.is_dir() {
        FileState::Dir
    } else {
        FileState::File
    }
}

/// `<dir>/<name>.<ext>` items, one folder level of namespacing
/// (`ns/name.md` → `ns/name`), `.disabled` suffix marks a disabled item.
pub fn scan_file_dir(dir: &Path, exts: &[&str], prefix: Option<&str>) -> Vec<FoundFile> {
    let mut found = Vec::new();
    collect_files(dir, exts, prefix, None, &mut found);
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn collect_files(
    dir: &Path,
    exts: &[&str],
    prefix: Option<&str>,
    namespace: Option<&str>,
    found: &mut Vec<FoundFile>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if path.is_dir() && namespace.is_none() {
            collect_files(&path, exts, prefix, Some(file_name), found);
            continue;
        }
        let Some((stem, enabled)) = match_item(file_name, exts) else {
            continue;
        };
        if prefix.is_some_and(|p| !stem.starts_with(p)) {
            continue;
        }
        let name = match namespace {
            Some(ns) => format!("{ns}/{stem}"),
            None => stem.to_owned(),
        };
        let meta = metadata::read(&path);
        let modified_at = mtime_unix(&path);
        found.push(FoundFile {
            name,
            path,
            enabled,
            meta,
            modified_at,
        });
    }
}

/// `name.ext` → enabled, `name.ext.disabled` → disabled, anything else → no.
fn match_item<'a>(file_name: &'a str, exts: &[&str]) -> Option<(&'a str, bool)> {
    let (base, enabled) = match file_name.strip_suffix(".disabled") {
        Some(base) => (base, false),
        None => (file_name, true),
    };
    for ext in exts {
        if let Some(stem) = base.strip_suffix(&format!(".{ext}"))
            && !stem.is_empty()
        {
            return Some((stem, enabled));
        }
    }
    None
}

/// `<dir>/<name>/<marker>` items; `<marker>.disabled` marks a disabled item.
pub fn scan_subdirs(dir: &Path, marker: &str) -> Vec<FoundFile> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let active = path.join(marker);
        let disabled = path.join(format!("{marker}.disabled"));
        let (marker_path, enabled) = if active.is_file() {
            (active, true)
        } else if disabled.is_file() {
            (disabled, false)
        } else {
            continue;
        };
        let meta = metadata::read(&marker_path);
        found.push(FoundFile {
            name: name.to_owned(),
            meta,
            modified_at: mtime_unix(&marker_path),
            path,
            enabled,
        });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Every `<dir>/*.<ext>` document, sorted by name. A `.disabled` suffix
/// takes a file out of the list the same way it does everywhere else: the
/// harness globs one extension, so the renamed file is no longer loaded.
pub fn scan_documents(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().and_then(|e| e.to_str()) == Some(ext))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_suffix_and_namespacing() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.md"), "").unwrap();
        fs::write(tmp.path().join("b.md.disabled"), "").unwrap();
        fs::write(tmp.path().join("noise.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("ns")).unwrap();
        fs::write(tmp.path().join("ns/c.md"), "").unwrap();

        let found = scan_file_dir(tmp.path(), &["md"], None);
        let names: Vec<_> = found.iter().map(|f| (f.name.as_str(), f.enabled)).collect();
        assert_eq!(names, [("a", true), ("b", false), ("ns/c", true)]);
        assert!(found.iter().all(|f| f.modified_at.is_some()));
    }

    #[test]
    fn mtime_reads_a_real_files_stat_and_reports_none_for_a_missing_one() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a.md");
        fs::write(&path, "").unwrap();
        let before = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32
            - 1;

        assert!(mtime_unix(&path).unwrap() >= before);
        assert_eq!(mtime_unix(&tmp.path().join("gone.md")), None);
    }

    #[test]
    fn subdir_items_require_the_marker() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("real")).unwrap();
        fs::write(
            tmp.path().join("real/SKILL.md"),
            "---\ndescription: x\n---\n",
        )
        .unwrap();
        fs::create_dir(tmp.path().join("off")).unwrap();
        fs::write(tmp.path().join("off/SKILL.md.disabled"), "").unwrap();
        fs::create_dir(tmp.path().join("junk")).unwrap();

        let found = scan_subdirs(tmp.path(), "SKILL.md");
        let names: Vec<_> = found.iter().map(|f| (f.name.as_str(), f.enabled)).collect();
        assert_eq!(names, [("off", false), ("real", true)]);
        assert_eq!(found[1].meta.description.as_deref(), Some("x"));
        assert!(found[1].modified_at.is_some());
    }
}
