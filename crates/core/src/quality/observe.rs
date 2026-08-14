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

use super::{
    AuditInput, Content, McpEntry, PluginSources, TreeFile, UNREAD_MCP_ENTRY, UNREADABLE_PLUGIN,
};

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
        ItemKind::McpServer => read_mcp(&item.path, &item.name),
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

/// Decoded the same way a plan's own bytes are: lossily, so one byte that
/// is not text cannot make a whole file invisible to every rule. What had to
/// be replaced is reported by `undecodable-content`.
fn read_document(path: &Path) -> Content {
    match std::fs::read(path) {
        Ok(bytes) => Content::Document {
            text: String::from_utf8_lossy(&bytes).into_owned(),
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

/// The server entry a harness would launch, dug back out of the config file
/// that holds it.
///
/// The scan reaches this file to learn the server's *name*; reading it again
/// for the command line is what lets the MCP rules run at all. Every layout
/// vstack writes nests the servers under one key and each server under its
/// own name, so the same walk covers JSON, JSONC and TOML. Where the entry
/// cannot be found the input says so and the rules report themselves not
/// applicable, which is the honest answer and never a pass.
fn read_mcp(path: &Path, name: &str) -> Content {
    const NESTS: &[&str] = &["mcpServers", "mcp_servers", "servers", "mcp"];
    let Some(root) = read_config(path) else {
        return Content::Unread {
            why: UNREAD_MCP_ENTRY,
        };
    };
    let entry = NESTS
        .iter()
        .filter_map(|nest| root.get(nest))
        .find_map(|table| table.get(name));
    match entry {
        Some(value) => Content::Mcp(McpEntry::from_json(value)),
        None => Content::Unread {
            why: UNREAD_MCP_ENTRY,
        },
    }
}

/// A config file as JSON, whichever of the two syntaxes it is written in.
fn read_config(path: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => toml::from_str::<serde_json::Value>(&text).ok(),
        _ => serde_json::from_str(&crate::scan::jsonc::to_json(&text)).ok(),
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
        let taken = char_boundary(&bytes, bytes.len().min(*budget));
        *budget -= taken;
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        files.push(TreeFile::read(relative.to_path_buf(), &bytes[..taken]));
    }
}

/// `at`, moved back to the nearest character boundary. Cutting a tree off
/// mid-character would leave bytes that will not decode, and those are now
/// reported — a budget the scanner chose is not the file's fault.
fn char_boundary(bytes: &[u8], at: usize) -> usize {
    let mut at = at;
    while at > 0 && at < bytes.len() && bytes[at] & 0xC0 == 0x80 {
        at -= 1;
    }
    at
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
