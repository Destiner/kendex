use std::path::Path;

use super::{RawEntry, hooks, jsonc, plugins};
use crate::env::Env;
use crate::fs::read_if_exists;
use crate::harness::Reader;

pub fn read_structured(path: &Path, reader: &Reader, env: &Env) -> Result<Vec<RawEntry>, String> {
    match reader {
        Reader::McpServersJson | Reader::ClaudeUserMcp => {
            Ok(mcp_object(read_json(path)?.get("mcpServers")))
        }
        Reader::ClaudeUserProjectMcp { project } => {
            let value = read_json(path)?;
            let servers = value
                .get("projects")
                .and_then(|p| p.get(project.to_string_lossy().as_ref()))
                .and_then(|p| p.get("mcpServers"));
            Ok(mcp_object(servers))
        }
        Reader::McpServersToml => mcp_toml(path),
        Reader::OpencodeMcp => opencode_mcp(path),
        Reader::OpencodePluginRefs => opencode_plugin_refs(path),
        Reader::HooksObject => hooks::read(path),
        Reader::ClaudePluginRegistry => plugins::claude_registry(path, env),
        Reader::ClaudeSettingsPlugins => plugins::claude_settings(path),
        Reader::CodexPluginCache => plugins::codex_cache(path),
        Reader::CursorPluginDirs => Ok(plugins::cursor_dirs(path)),
        Reader::PiPackages => pi_packages(path),
    }
}

/// jsonc-tolerant read: comments and trailing commas never block a scan.
pub fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let text = read_if_exists(path)
        .map_err(|e| e.to_string())?
        .ok_or("file vanished mid-scan")?;
    serde_json::from_str(&jsonc::to_json(&text)).map_err(|e| e.to_string())
}

fn mcp_object(servers: Option<&serde_json::Value>) -> Vec<RawEntry> {
    let Some(map) = servers.and_then(|s| s.as_object()) else {
        return Vec::new();
    };
    map.iter()
        .map(|(name, entry)| RawEntry {
            name: name.clone(),
            enabled: None,
            description: mcp_summary(entry),
        })
        .collect()
}

/// The command or URL — how a list view tells servers apart.
fn mcp_summary(entry: &serde_json::Value) -> Option<String> {
    for key in ["command", "url"] {
        if let Some(value) = entry.get(key).and_then(|v| v.as_str()) {
            return Some(value.to_owned());
        }
    }
    entry
        .get("command")
        .and_then(|c| c.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
}

fn mcp_toml(path: &Path) -> Result<Vec<RawEntry>, String> {
    let text = read_if_exists(path)
        .map_err(|e| e.to_string())?
        .ok_or("file vanished mid-scan")?;
    let value: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let Some(servers) = value.get("mcp_servers").and_then(|s| s.as_table()) else {
        return Ok(Vec::new());
    };
    Ok(servers
        .iter()
        .map(|(name, entry)| RawEntry {
            name: name.clone(),
            enabled: None,
            description: entry
                .get("command")
                .or_else(|| entry.get("url"))
                .and_then(|v| v.as_str())
                .map(str::to_owned),
        })
        .collect())
}

fn opencode_mcp(path: &Path) -> Result<Vec<RawEntry>, String> {
    let value = read_json(path)?;
    let Some(map) = value.get("mcp").and_then(|m| m.as_object()) else {
        return Ok(Vec::new());
    };
    Ok(map
        .iter()
        .map(|(name, entry)| RawEntry {
            name: name.clone(),
            enabled: Some(
                entry
                    .get("enabled")
                    .and_then(|e| e.as_bool())
                    .unwrap_or(true),
            ),
            description: mcp_summary(entry),
        })
        .collect())
}

fn opencode_plugin_refs(path: &Path) -> Result<Vec<RawEntry>, String> {
    let value = read_json(path)?;
    let Some(refs) = value.get("plugin").and_then(|p| p.as_array()) else {
        return Ok(Vec::new());
    };
    Ok(refs
        .iter()
        .filter_map(|r| r.as_str())
        .map(|spec| RawEntry {
            name: spec.to_owned(),
            enabled: None,
            description: Some("npm plugin ref".to_owned()),
        })
        .collect())
}

fn pi_packages(path: &Path) -> Result<Vec<RawEntry>, String> {
    let value = read_json(path)?;
    let Some(packages) = value.get("packages").and_then(|p| p.as_array()) else {
        return Ok(Vec::new());
    };
    Ok(packages
        .iter()
        .filter_map(|entry| match entry {
            serde_json::Value::String(spec) => Some(spec.clone()),
            other => other
                .get("source")
                .and_then(|s| s.as_str())
                .map(str::to_owned),
        })
        .map(|spec| RawEntry {
            name: pi_package_name(&spec),
            enabled: None,
            description: Some(spec),
        })
        .collect())
}

/// `npm:@scope/pkg@1.0` → `@scope/pkg`, `./packages/x` → `x`,
/// `https://host/a/b` → `b`, anything else verbatim.
fn pi_package_name(spec: &str) -> String {
    if let Some(rest) = spec.strip_prefix("npm:") {
        let version_at = match rest.strip_prefix('@') {
            Some(scoped) => scoped.find('@').map(|i| i + 1),
            None => rest.find('@'),
        };
        return match version_at {
            Some(i) => rest[..i].to_owned(),
            None => rest.to_owned(),
        };
    }
    if spec.contains('/')
        && let Some(last) = spec.trim_end_matches('/').rsplit('/').next()
    {
        return last.to_owned();
    }
    spec.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_package_names_cover_every_spec_shape() {
        assert_eq!(
            pi_package_name("npm:@vanillagreen/pi-hooks@1.2.0"),
            "@vanillagreen/pi-hooks"
        );
        assert_eq!(pi_package_name("npm:plain@2"), "plain");
        assert_eq!(pi_package_name("npm:plain"), "plain");
        assert_eq!(pi_package_name("./packages/pi-tmux"), "pi-tmux");
        assert_eq!(pi_package_name("https://github.com/a/b"), "b");
        assert_eq!(pi_package_name("odd"), "odd");
    }

    #[test]
    fn codex_mcp_toml_lists_server_names() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            "[mcp_servers.github]\ncommand = \"gh-mcp\"\n[mcp_servers.db]\nurl = \"https://x\"\n",
        )
        .unwrap();
        let mut entries = mcp_toml(&path).unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(entries[0].name, "db");
        assert_eq!(entries[1].description.as_deref(), Some("gh-mcp"));
    }

    #[test]
    fn opencode_mcp_honors_enabled_and_jsonc() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("opencode.jsonc");
        std::fs::write(
            &path,
            r#"{
  // servers
  "mcp": {
    "on": {"type": "remote", "url": "https://x"},
    "off": {"type": "local", "command": ["db", "run"], "enabled": false},
  },
}"#,
        )
        .unwrap();
        let mut entries = opencode_mcp(&path).unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(
            entries
                .iter()
                .map(|e| (e.name.as_str(), e.enabled))
                .collect::<Vec<_>>(),
            [("off", Some(false)), ("on", Some(true))]
        );
        assert_eq!(entries[0].description.as_deref(), Some("db run"));
    }
}
