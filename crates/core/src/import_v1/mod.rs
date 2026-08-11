use std::collections::BTreeMap;

use toml::Value;

use crate::lock::{LOCK_VERSION, Lock, LockEntry};
use crate::manifest::{
    DEFAULT_SOURCE_NAME, DEFAULT_SOURCE_REPO, FrontmatterOverrides, ItemDecl, MANIFEST_SCHEMA,
    Manifest, Method, SourceDecl,
};
use crate::model::{HarnessId, ItemKind};

#[derive(Debug)]
pub struct ImportOutcome {
    pub manifest: Manifest,
    pub lock: Lock,
    pub notes: Vec<String>,
}

/// One-shot v1 → v2 conversion of a scope's manifest + lock text. Pure:
/// callers read the files, back the originals up, and write the results
/// through a plan.
pub fn convert(v1_manifest: Option<&str>, v1_lock: Option<&str>) -> Result<ImportOutcome, String> {
    let mut notes = Vec::new();
    let mut manifest = Manifest {
        schema: MANIFEST_SCHEMA,
        ..Manifest::default()
    };
    let mut lock = Lock {
        version: LOCK_VERSION,
        entries: BTreeMap::new(),
    };

    if let Some(text) = v1_manifest {
        let table: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
        convert_tables(&table, &mut manifest, &mut notes);
    }
    if let Some(text) = v1_lock {
        convert_lock(text, &mut manifest, &mut lock, &mut notes)?;
    }
    Ok(ImportOutcome {
        manifest,
        lock,
        notes,
    })
}

fn string_map(table: &toml::Table, keys: &[&str]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for key in keys {
        let Some(section) = table.get(*key).and_then(Value::as_table) else {
            continue;
        };
        for (name, value) in section {
            if let Some(text) = value.as_str().filter(|t| !t.trim().is_empty()) {
                out.insert(name.clone(), text.to_owned());
            }
        }
    }
    out
}

fn convert_tables(table: &toml::Table, manifest: &mut Manifest, notes: &mut Vec<String>) {
    if let Some(skills) = table.get("agent-skills").and_then(Value::as_table) {
        for (agent, list) in skills {
            let entries: Vec<String> = list
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            manifest.agent_skills.insert(agent.clone(), entries);
        }
    }
    manifest.agent_launch_instructions =
        string_map(table, &["agent-launch-instructions", "agent-guidance"]);
    manifest.agent_additional_instructions = string_map(
        table,
        &["agent-additional-instructions", "agent-instructions"],
    );
    manifest.skill_instructions = string_map(table, &["skill-instructions"]);
    if let Some(dir) = table.get("project-skills-dir").and_then(Value::as_str) {
        manifest.project_skills_dir = Some(dir.to_owned());
    }
    if table.get("agent-colors").is_some() {
        notes.push(
            "dropped deprecated [agent-colors] — set color in [agent-frontmatter.<harness>.<agent>]"
                .to_owned(),
        );
    }
    if let Some(hooks) = table.get("custom-hooks").and_then(Value::as_array) {
        for hook in hooks {
            if let Ok(parsed) = hook.clone().try_into() {
                manifest.custom_hooks.push(parsed);
            } else {
                notes.push("skipped one malformed [[custom-hooks]] entry".to_owned());
            }
        }
    }
    convert_frontmatter(table, manifest, notes);
}

const HARNESS_IDS: [&str; 5] = ["claude-code", "codex", "opencode", "cursor", "pi"];

fn convert_frontmatter(table: &toml::Table, manifest: &mut Manifest, notes: &mut Vec<String>) {
    let Some(frontmatter) = table.get("agent-frontmatter").and_then(Value::as_table) else {
        return;
    };
    for (key, agents) in frontmatter {
        let Some(harness) = HarnessId::parse(key) else {
            notes.push(format!(
                "dropped legacy harness-agnostic [agent-frontmatter.{key}] — v2 overrides are per-harness"
            ));
            continue;
        };
        let Some(agents) = agents.as_table() else {
            continue;
        };
        let mut per_agent = BTreeMap::new();
        for (agent, overrides) in agents {
            if let Some(overrides) = overrides.as_table() {
                per_agent.insert(agent.clone(), convert_overrides(overrides, notes));
            }
        }
        manifest
            .agent_frontmatter
            .insert(harness.name().to_owned(), per_agent);
    }
    let _ = HARNESS_IDS;
}

/// v1 accepted several aliases per field and a legacy `tools` allowlist;
/// v2 keeps one spelling and the deny-only model.
fn convert_overrides(table: &toml::Table, notes: &mut Vec<String>) -> FrontmatterOverrides {
    let text = |key: &str| table.get(key).and_then(Value::as_str).map(str::to_owned);
    let flag = |key: &str| table.get(key).and_then(Value::as_bool);
    let list = |keys: &[&str]| {
        keys.iter().find_map(|key| {
            table.get(*key).map(|value| match value {
                Value::String(csv) => csv
                    .split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect(),
                Value::Array(items) => items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                _ => Vec::new(),
            })
        })
    };
    if table.contains_key("tools") {
        notes.push("dropped legacy `tools` allowlist — v2 is deny-only".to_owned());
    }
    FrontmatterOverrides {
        color: text("color"),
        model: text("model"),
        deny_tools: list(&["deny-tools", "deny_tools"]),
        allowed_subagents: list(&[
            "allowed-subagents",
            "allowedSubagents",
            "subagent-agents",
            "subagent_agents",
        ]),
        pane: flag("pane"),
        background: flag("background"),
        effort: text("effort"),
        isolation: text("isolation"),
        memory: text("memory"),
        mode: text("mode"),
        sandbox_mode: text("sandbox-mode"),
        model_reasoning_effort: text("model-reasoning-effort"),
        nickname_candidates: list(&[
            "nickname-candidates",
            "nicknameCandidates",
            "nickname_candidates",
        ]),
    }
}

fn source_name_for(source: &str, source_repo: Option<&str>) -> (String, SourceDecl) {
    let reference = source_repo.unwrap_or(source);
    if reference == DEFAULT_SOURCE_REPO {
        return (
            DEFAULT_SOURCE_NAME.to_owned(),
            SourceDecl {
                repo: Some(DEFAULT_SOURCE_REPO.to_owned()),
                path: None,
                enabled: true,
            },
        );
    }
    let is_repo = reference.contains('/')
        && !reference.starts_with('.')
        && !reference.starts_with('/')
        && !reference.starts_with('~')
        && reference.matches('/').count() == 1;
    let name = reference
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(reference)
        .to_owned();
    let decl = if is_repo {
        SourceDecl {
            repo: Some(reference.to_owned()),
            path: None,
            enabled: true,
        }
    } else {
        SourceDecl {
            repo: None,
            path: Some(reference.to_owned()),
            enabled: true,
        }
    };
    (name, decl)
}

fn convert_lock(
    text: &str,
    manifest: &mut Manifest,
    lock: &mut Lock,
    notes: &mut Vec<String>,
) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let Some(entries) = value.get("entries").and_then(|e| e.as_object()) else {
        return Ok(());
    };
    for (name, entry) in entries {
        let kind_text = entry.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let kind = match kind_text {
            "skill" => ItemKind::Skill,
            "agent" => ItemKind::Agent,
            "hook" => ItemKind::Hook,
            "pi-extension" => ItemKind::PiExtension,
            "extra" => {
                notes.push(format!(
                    "skipped extra '{name}' — theme packs are gone in v2"
                ));
                continue;
            }
            other => {
                notes.push(format!("skipped '{name}' with unknown kind '{other}'"));
                continue;
            }
        };
        let source = entry.get("source").and_then(|s| s.as_str()).unwrap_or("");
        let source_repo = entry.get("source_repo").and_then(|s| s.as_str());
        let (source_name, decl) = source_name_for(source, source_repo);
        manifest.sources.entry(source_name.clone()).or_insert(decl);
        manifest
            .declared_mut(kind)
            .entry(name.clone())
            .or_insert_with(|| ItemDecl::from_source(&source_name));

        let method = match entry.get("method").and_then(|m| m.as_str()) {
            Some("copy") => Method::Copy,
            _ => Method::Symlink,
        };
        let installed_at = entry
            .get("installed_at")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_owned();
        let harnesses: Vec<HarnessId> = entry
            .get("harnesses")
            .and_then(|h| h.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|h| h.as_str())
                    .filter_map(HarnessId::parse)
                    .collect()
            })
            .unwrap_or_default();
        for harness in &harnesses {
            lock.entries.insert(
                crate::lock::entry_key(kind, name, *harness),
                LockEntry {
                    name: name.clone(),
                    kind,
                    harness: *harness,
                    source: source_name.clone(),
                    source_repo: source_repo.unwrap_or(source).to_owned(),
                    method,
                    installed_at: installed_at.clone(),
                    // v1 hashes are FNV — never comparable, so the first
                    // refresh regenerates everything, by design.
                    source_hash: format!(
                        "v1:{}",
                        entry
                            .get("source_hash")
                            .and_then(|h| h.as_str())
                            .unwrap_or("")
                    ),
                    enabled: true,
                    upstream_skills: None,
                },
            );
        }
        if !manifest.install.harnesses.is_empty() {
            continue;
        }
        manifest.install.harnesses = harnesses;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
