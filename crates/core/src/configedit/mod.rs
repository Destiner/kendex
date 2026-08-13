use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

mod copilot;

use copilot::{remove_copilot_hook, upsert_copilot_hook};

/// A deterministic, idempotent structured edit. Applied to the file's
/// current text at execute time; a file is in sync exactly when
/// `apply(current) == current` — that equality is the drift check for
/// config-entry kinds. Unrelated keys always survive (invariant 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigEdit {
    /// claude settings.json / codex hooks.json: upsert our handler under
    /// `hooks.<event>` for `matcher`, replacing entries that run `command`.
    UpsertHook {
        event: String,
        matcher: Option<String>,
        command: String,
        timeout: Option<u32>,
    },
    /// Remove our handler (from every event when `event` is None); empty
    /// groups and events are pruned.
    RemoveHook {
        event: Option<String>,
        command: String,
    },
    /// copilot hook file: upsert our entry under `hooks.<event>`, replacing
    /// any entry that already runs `command`. Copilot's entries carry the
    /// command and the matcher themselves and the document declares the
    /// schema version it was written for, so none of the shape above fits
    /// (docs.github.com/en/copilot/reference/hooks-reference).
    UpsertCopilotHook {
        event: String,
        matcher: Option<String>,
        command: String,
        timeout: Option<u32>,
    },
    RemoveCopilotHook {
        event: Option<String>,
        command: String,
    },
    /// `mcpServers.<name>` upsert with a full value.
    UpsertMcpServer {
        name: String,
        value: Value,
    },
    RemoveMcpServer {
        name: String,
    },
    /// `enabledPlugins.<key>` set/remove.
    SetPluginEnabled {
        key: String,
        enabled: Option<bool>,
    },
    /// gemini `mcp-server-enablement.json`, whose whole content is
    /// `{"<server>": {"enabled": bool}}` — one global file recording
    /// whether a server is on, wherever it was declared (matrix §1).
    SetGeminiMcpEnabled {
        name: String,
        enabled: Option<bool>,
    },
    /// opencode.json: ensure `instructions[]` carries `reference`; for
    /// PreToolUse:Bash hooks also `permission.bash = {"*": "ask"}`.
    OpencodeAddInstruction {
        reference: String,
        bash_permission: bool,
    },
    OpencodeRemoveInstruction {
        reference: String,
    },
    /// codex config.toml: text-level `[features] hooks = true` merge that
    /// preserves comments and ordering.
    CodexEnableHooksFeature,
    /// APPEND_SYSTEM.md-style marker block upsert/removal.
    UpsertMarkerBlock {
        name: String,
        block: String,
    },
    RemoveMarkerBlock {
        name: String,
    },
}

impl ConfigEdit {
    pub fn apply(&self, current: &str) -> Result<String, String> {
        match self {
            ConfigEdit::CodexEnableHooksFeature => Ok(codex_enable_hooks(current)),
            ConfigEdit::UpsertMarkerBlock { name, block } => {
                Ok(upsert_marker_block(current, name, block))
            }
            ConfigEdit::RemoveMarkerBlock { name } => Ok(remove_marker_block(current, name)),
            json_edit => {
                let mut root: Value = if current.trim().is_empty() {
                    json!({})
                } else {
                    serde_json::from_str(current).map_err(|e| e.to_string())?
                };
                json_edit.apply_json(&mut root)?;
                let mut text = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
                text.push('\n');
                Ok(text)
            }
        }
    }

    fn apply_json(&self, root: &mut Value) -> Result<(), String> {
        let object = root
            .as_object_mut()
            .ok_or("config root is not a JSON object")?;
        match self {
            ConfigEdit::UpsertHook {
                event,
                matcher,
                command,
                timeout,
            } => upsert_hook(object, event, matcher.as_deref(), command, *timeout),
            ConfigEdit::RemoveHook { event, command } => {
                let events: Vec<String> = match event {
                    Some(event) => vec![event.clone()],
                    None => object
                        .get("hooks")
                        .and_then(Value::as_object)
                        .map(|e| e.keys().cloned().collect())
                        .unwrap_or_default(),
                };
                for event in events {
                    remove_hook(object, &event, command);
                }
                Ok(())
            }
            ConfigEdit::UpsertCopilotHook {
                event,
                matcher,
                command,
                timeout,
            } => upsert_copilot_hook(object, event, matcher.as_deref(), command, *timeout),
            ConfigEdit::RemoveCopilotHook { event, command } => {
                remove_copilot_hook(object, event.as_deref(), command);
                Ok(())
            }
            ConfigEdit::UpsertMcpServer { name, value } => {
                let servers = ensure_object(object, "mcpServers")?;
                servers.insert(name.clone(), value.clone());
                Ok(())
            }
            ConfigEdit::RemoveMcpServer { name } => {
                remove_from_map(object, "mcpServers", name);
                Ok(())
            }
            ConfigEdit::SetPluginEnabled { key, enabled } => {
                match enabled {
                    Some(enabled) => {
                        ensure_object(object, "enabledPlugins")?
                            .insert(key.clone(), Value::Bool(*enabled));
                    }
                    None => remove_from_map(object, "enabledPlugins", key),
                }
                Ok(())
            }
            ConfigEdit::SetGeminiMcpEnabled { name, enabled } => {
                set_gemini_mcp_enabled(object, name, *enabled)
            }
            ConfigEdit::OpencodeAddInstruction {
                reference,
                bash_permission,
            } => opencode_add_instruction(object, reference, *bash_permission),
            ConfigEdit::OpencodeRemoveInstruction { reference } => {
                if let Some(list) = object.get_mut("instructions").and_then(Value::as_array_mut) {
                    list.retain(|v| v.as_str() != Some(reference));
                    if list.is_empty() {
                        object.remove("instructions");
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// Drop one entry from a map, and the map itself once it holds nothing —
/// an empty object left behind is a key the user never wrote.
fn remove_from_map(object: &mut Map<String, Value>, key: &str, entry: &str) {
    if let Some(map) = object.get_mut(key).and_then(Value::as_object_mut) {
        map.remove(entry);
        if map.is_empty() {
            object.remove(key);
        }
    }
}

fn opencode_add_instruction(
    object: &mut Map<String, Value>,
    reference: &str,
    bash_permission: bool,
) -> Result<(), String> {
    if object.is_empty() {
        object.insert(
            "$schema".into(),
            Value::String("https://opencode.ai/config.json".into()),
        );
    }
    let list = object
        .entry("instructions")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or("instructions is not an array")?;
    if !list.iter().any(|v| v.as_str() == Some(reference)) {
        list.push(Value::String(reference.to_owned()));
    }
    if bash_permission {
        let permission = ensure_object(object, "permission")?;
        permission
            .entry("bash")
            .or_insert_with(|| json!({"*": "ask"}));
    }
    Ok(())
}

fn ensure_object<'a>(
    object: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    object
        .entry(key)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(format!("{key} is not an object"))
}

/// The whole file is a map of server name to its state, so clearing our
/// entry takes the name with it rather than leaving an empty object behind.
fn set_gemini_mcp_enabled(
    root: &mut Map<String, Value>,
    name: &str,
    enabled: Option<bool>,
) -> Result<(), String> {
    match enabled {
        Some(enabled) => {
            ensure_object(root, name)?.insert("enabled".into(), Value::Bool(enabled));
        }
        None => {
            root.remove(name);
        }
    }
    Ok(())
}

fn upsert_hook(
    root: &mut Map<String, Value>,
    event: &str,
    matcher: Option<&str>,
    command: &str,
    timeout: Option<u32>,
) -> Result<(), String> {
    remove_hook(root, event, command);
    let hooks = ensure_object(root, "hooks")?;
    let groups = hooks
        .entry(event)
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or("hook event is not an array")?;
    let mut handler = json!({"type": "command", "command": command});
    if let Some(timeout) = timeout {
        handler["timeout"] = json!(timeout);
    }
    let group = groups.iter_mut().find(|g| {
        g.get("matcher").and_then(Value::as_str) == matcher
            || (matcher.is_none() && g.get("matcher").is_none())
    });
    match group {
        Some(group) => {
            let handlers = group
                .as_object_mut()
                .and_then(|g| g.get_mut("hooks"))
                .and_then(Value::as_array_mut)
                .ok_or("hook group has no handler array")?;
            handlers.push(handler);
        }
        None => {
            let mut group = Map::new();
            if let Some(matcher) = matcher {
                group.insert("matcher".into(), Value::String(matcher.to_owned()));
            }
            group.insert("hooks".into(), Value::Array(vec![handler]));
            groups.push(Value::Object(group));
        }
    }
    Ok(())
}

fn remove_hook(root: &mut Map<String, Value>, event: &str, command: &str) {
    let Some(events) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return;
    };
    if let Some(groups) = events.get_mut(event).and_then(Value::as_array_mut) {
        for group in groups.iter_mut() {
            if let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                handlers.retain(|h| h.get("command").and_then(Value::as_str) != Some(command));
            }
        }
        groups.retain(|group| {
            group
                .get("hooks")
                .and_then(Value::as_array)
                .is_none_or(|handlers| !handlers.is_empty())
        });
        if groups.is_empty() {
            events.remove(event);
        }
    }
    if events.is_empty() {
        root.remove("hooks");
    }
}

/// Preserves comments and ordering: appends to an existing `[features]`
/// section or adds one at the end. Deprecated `codex_hooks` keys migrate.
fn codex_enable_hooks(current: &str) -> String {
    if current
        .lines()
        .any(|line| line.trim() == "hooks = true" || line.trim().starts_with("hooks=true"))
    {
        return current.replace("codex_hooks", "hooks");
    }
    let text = current.replace("codex_hooks", "hooks");
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    if let Some(position) = lines.iter().position(|l| l.trim() == "[features]") {
        lines.insert(position + 1, "hooks = true".to_owned());
    } else {
        if !lines.is_empty() && !lines.last().is_some_and(|l| l.is_empty()) {
            lines.push(String::new());
        }
        lines.push("[features]".to_owned());
        lines.push("hooks = true".to_owned());
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn marker_bounds(name: &str) -> (String, String) {
    (
        format!("<!-- vstack:append-system {name} begin -->"),
        format!("<!-- vstack:append-system {name} end -->"),
    )
}

pub fn upsert_marker_block(current: &str, name: &str, block: &str) -> String {
    let stripped = remove_marker_block(current, name);
    let (begin, end) = marker_bounds(name);
    let base = stripped.trim_end();
    if base.is_empty() {
        format!("{begin}\n{block}\n{end}\n")
    } else {
        format!("{base}\n\n{begin}\n{block}\n{end}\n")
    }
}

pub fn remove_marker_block(current: &str, name: &str) -> String {
    let (begin, end) = marker_bounds(name);
    let Some(start) = current.find(&begin) else {
        return current.to_owned();
    };
    let Some(stop) = current[start..].find(&end) else {
        // Unterminated markers are user damage; leave them untouched.
        return current.to_owned();
    };
    let before = current[..start].trim_end_matches('\n');
    let after = current[start + stop + end.len()..].trim_start_matches('\n');
    match (before.is_empty(), after.is_empty()) {
        (true, true) => String::new(),
        (true, false) => after.to_owned(),
        (false, true) => format!("{before}\n"),
        (false, false) => format!("{before}\n\n{after}"),
    }
}

#[cfg(test)]
mod tests;
