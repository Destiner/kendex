//! The second scoring path: what is on disk right now, rather than what a
//! plan would write. Same rules, a different set of bytes.
//!
//! An installed item is not the same thing as a desired one. It may predate
//! the declaration, it may have been edited by hand, and it may not be
//! declared at all — that is exactly what an audit is for. Where the bytes
//! cannot be reached from an observation alone (an MCP server that lives as
//! one entry inside a shared config file, a plugin whose own directory the
//! scanner never visits), the input says so and every rule that would have
//! read them reports itself not applicable.

use std::path::Path;

use crate::model::{ItemKind, ObservedItem};

use super::{AuditInput, Content, PluginSources, TreeFile, UNREAD_MCP_ENTRY, UNREADABLE_PLUGIN};

/// Total bytes read from one tree, and the number of files. A hostile or
/// merely enormous tree must not turn an audit into a memory problem.
const MAX_TREE_BYTES: usize = 512 * 1024;
const MAX_TREE_FILES: usize = 200;

/// What this observation carries, read as an audit input.
pub fn input_for(item: &ObservedItem) -> AuditInput {
    let location = item.path.display().to_string();
    let content = match item.kind {
        ItemKind::Skill => read_tree(&item.path),
        ItemKind::Agent | ItemKind::Command | ItemKind::PiExtension => read_document(&item.path),
        ItemKind::Hook => read_hook(&item.path),
        ItemKind::McpServer => Content::Unread {
            why: UNREAD_MCP_ENTRY,
        },
        ItemKind::Plugin => read_plugin(&item.path),
    };
    AuditInput {
        kind: item.kind,
        name: item.name.clone(),
        harness: Some(item.harness),
        location,
        content,
    }
}

const UNREADABLE_FILE: &str = "the installed file could not be read from disk";
const NOT_A_TREE: &str = "the installed skill is not a directory on disk";

fn read_document(path: &Path) -> Content {
    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Content::Document { text },
            Err(_) => Content::Unread {
                why: "the installed file is not text",
            },
        },
        Err(_) => Content::Unread {
            why: UNREADABLE_FILE,
        },
    }
}

fn read_hook(path: &Path) -> Content {
    let Content::Document { text } = read_document(path) else {
        return Content::Unread {
            why: UNREADABLE_FILE,
        };
    };
    Content::Hook {
        event: String::new(),
        matcher: None,
        command: path.display().to_string(),
        script: Some(text),
    }
}

fn read_tree(root: &Path) -> Content {
    if !root.is_dir() {
        return Content::Unread { why: NOT_A_TREE };
    }
    let mut files = Vec::new();
    let mut budget = MAX_TREE_BYTES;
    walk(root, root, &mut files, &mut budget);
    files.sort_by(|a: &TreeFile, b: &TreeFile| a.path.cmp(&b.path));
    Content::SkillTree { files }
}

/// Depth-first, budget-bounded, and never through a symlink: the canonical
/// tree is the one vstack wrote, and following a link out of it would audit
/// somebody else's files under this item's name.
fn walk(root: &Path, dir: &Path, files: &mut Vec<TreeFile>, budget: &mut usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        if files.len() >= MAX_TREE_FILES || *budget == 0 {
            return;
        }
        if path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, files, budget);
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let taken = bytes.len().min(*budget);
        *budget -= taken;
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        files.push(TreeFile::read(relative.to_path_buf(), &bytes[..taken]));
    }
}

/// A plugin directory, when the observation points at one. The scanner
/// reads plugins out of registries and settings files, so most of the time
/// the path is a config file and the plugin's own sources are elsewhere.
fn read_plugin(path: &Path) -> Content {
    let root = match path.is_dir() {
        true => path,
        false => {
            return Content::Unread {
                why: UNREADABLE_PLUGIN,
            };
        }
    };
    const MANIFESTS: &[&str] = &[
        "plugin.json",
        "package.json",
        ".cursor-plugin/plugin.json",
        ".codex-plugin/plugin.json",
    ];
    let manifests: Vec<String> = MANIFESTS
        .iter()
        .filter(|name| root.join(name).is_file())
        .map(|name| (*name).to_owned())
        .collect();
    let Content::SkillTree { files } = read_tree(root) else {
        return Content::Unread {
            why: UNREADABLE_PLUGIN,
        };
    };
    Content::Plugin(PluginSources {
        package_json: std::fs::read_to_string(root.join("package.json")).ok(),
        git_origin: root
            .join(".git")
            .exists()
            .then(|| root.display().to_string()),
        scripts: files
            .into_iter()
            .filter(|file| is_source(&file.path))
            .collect(),
        manifests,
    })
}

fn is_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("js" | "ts" | "mjs" | "cjs" | "py" | "sh" | "bash")
    )
}
