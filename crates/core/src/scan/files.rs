use std::fs;
use std::path::{Path, PathBuf};

use crate::model::FileState;

pub struct FoundFile {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
    pub description: Option<String>,
}

pub fn state_of(path: &Path) -> FileState {
    if path.is_symlink() {
        let target = fs::read_link(path).unwrap_or_default();
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
        let description = read_description(&path);
        found.push(FoundFile {
            name,
            path,
            enabled,
            description,
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
        found.push(FoundFile {
            name: name.to_owned(),
            description: read_description(&marker_path),
            path,
            enabled,
        });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

const DESCRIPTION_PROBE_BYTES: usize = 4096;

/// Best-effort `description` from YAML frontmatter (md/mdc) or a top-level
/// TOML `description` key — enough for list views, not a full parser.
fn read_description(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    let text = read_head(path)?;
    match ext {
        "md" | "mdc" => frontmatter_description(&text),
        "toml" => toml_description(&text),
        _ => None,
    }
}

fn read_head(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let head = &bytes[..bytes.len().min(DESCRIPTION_PROBE_BYTES)];
    Some(String::from_utf8_lossy(head).into_owned())
}

fn frontmatter_description(text: &str) -> Option<String> {
    let body = text.strip_prefix("---")?;
    let end = body.find("\n---")?;
    for line in body[..end].lines() {
        if let Some(value) = line.strip_prefix("description:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn toml_description(text: &str) -> Option<String> {
    let value: toml::Table = text.parse().ok()?;
    value
        .get("description")
        .and_then(|d| d.as_str())
        .map(str::to_owned)
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
        assert_eq!(found[1].description.as_deref(), Some("x"));
    }

    #[test]
    fn frontmatter_description_handles_quotes_and_absence() {
        assert_eq!(
            frontmatter_description("---\ndescription: \"hi there\"\n---\nbody"),
            Some("hi there".to_owned())
        );
        assert_eq!(frontmatter_description("no frontmatter"), None);
    }
}
