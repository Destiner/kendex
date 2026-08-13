use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::desired::{Artifact, Desired, DesiredState, ItemCtx};
use super::targets::{
    HookFormat, HookTarget, disabled_name, hook_target, mcp_registry, plugin_settings,
};
use crate::configedit::ConfigEdit;
use crate::env::Env;
use crate::error::Result;
use crate::hash::{hash_bytes, installation_hash};
use crate::hook::{HookSource, codex_event, parse_hook};
use crate::lock::entry_key;
use crate::manifest::{Manifest, Method};
use crate::model::{HarnessId, ItemKind, Scope};

/// Hooks, commands, and MCP servers all declare the same way; only the
/// artifact differs.
pub(super) fn declared(
    ctx: &ItemCtx,
    kind: ItemKind,
    harness: HarnessId,
    artifact: Artifact,
) -> Result<Desired> {
    Ok(Desired {
        key: entry_key(kind, ctx.name, harness),
        kind,
        name: ctx.name.to_owned(),
        harness,
        enabled: ctx.decl.enabled,
        method: Method::Copy,
        source_name: ctx.decl.source.clone(),
        provenance: ctx.provenance.to_owned(),
        hash: installation_hash(
            ctx.sealed,
            ctx.item_path,
            ctx.manifest,
            kind,
            ctx.name,
            harness,
        )?,
        upstream_skills: None,
        emitted: None,
        artifact,
    })
}

pub(super) fn desired_hook(ctx: &ItemCtx, state: &mut DesiredState) -> Result<()> {
    let text = ctx.sealed.read_to_string(ctx.item_path)?;
    let hook = match parse_hook(&text) {
        Ok(hook) => hook,
        Err(problem) => {
            state.unreadable(
                ItemKind::Hook,
                ctx.name,
                format!("hook {}: unreadable — {problem}", ctx.name),
            );
            return Ok(());
        }
    };
    for harness in ctx.harnesses.clone() {
        if !hook.applies_to(harness) {
            state.notes.push(format!(
                "hook {}: not declared for {}",
                ctx.name,
                harness.name()
            ));
            continue;
        }
        if harness == HarnessId::Codex && codex_event(&hook.event).is_none() {
            state.notes.push(format!(
                "hook {}: event {} unsupported on codex — advisory prose lands with the customization editor",
                ctx.name, hook.event
            ));
            continue;
        }
        // Gemini and Copilot each name the lifecycle events their own way,
        // so the hook is restated in the reader's words — and whatever their
        // configuration already says about hooks is said now, before
        // anything is registered.
        let hook = match harness {
            HarnessId::Gemini => match super::gemini::hook(ctx, &hook, state) {
                Some(hook) => hook,
                None => continue,
            },
            HarnessId::Copilot => match super::copilot::hook(ctx, &hook, state) {
                Some(hook) => hook,
                None => continue,
            },
            _ => hook.clone(),
        };
        let Some(target) = hook_target(ctx.env, ctx.scope, harness, ctx.name) else {
            continue;
        };
        let artifact = hook_artifact(&target, &hook, ctx.name, ctx.decl.enabled);
        state
            .items
            .push(declared(ctx, ItemKind::Hook, harness, artifact)?);
    }
    Ok(())
}

/// A disabled hook keeps its file under the `.disabled` name and reverses its
/// registration — the constraint stops applying without losing anything.
fn hook_artifact(target: &HookTarget, hook: &HookSource, name: &str, enabled: bool) -> Artifact {
    let placed = |path: &PathBuf| {
        if enabled {
            path.clone()
        } else {
            disabled_name(path)
        }
    };
    match target {
        HookTarget::Script {
            path,
            command,
            registry,
            format,
            feature,
        } => {
            let registration = match (enabled, format) {
                (true, HookFormat::Nested) => ConfigEdit::UpsertHook {
                    event: hook.event.clone(),
                    matcher: hook.matcher.clone(),
                    command: command.clone(),
                    timeout: hook.timeout,
                },
                (true, HookFormat::Copilot) => ConfigEdit::UpsertCopilotHook {
                    event: hook.event.clone(),
                    matcher: hook.matcher.clone(),
                    command: command.clone(),
                    timeout: hook.timeout,
                },
                (false, HookFormat::Nested) => ConfigEdit::RemoveHook {
                    event: Some(hook.event.clone()),
                    command: command.clone(),
                },
                (false, HookFormat::Copilot) => ConfigEdit::RemoveCopilotHook {
                    event: Some(hook.event.clone()),
                    command: command.clone(),
                },
            };
            let mut edits = registration_edits(registry, registration, enabled);
            if let Some(feature) = feature
                && enabled
            {
                edits.push((feature.clone(), ConfigEdit::CodexEnableHooksFeature));
            }
            Artifact::Registration {
                script: Some((placed(path), hook.script.clone().into_bytes())),
                edits,
            }
        }
        HookTarget::Instruction {
            path,
            config,
            reference,
        } => {
            let edit = if enabled {
                ConfigEdit::OpencodeAddInstruction {
                    reference: reference.clone(),
                    bash_permission: hook.event == "PreToolUse"
                        && hook.matcher.as_deref() == Some("Bash"),
                }
            } else {
                ConfigEdit::OpencodeRemoveInstruction {
                    reference: reference.clone(),
                }
            };
            let body = format!("# Safety: {name}\n\n{}", hook.safety_prose());
            Artifact::Registration {
                script: Some((placed(path), body.into_bytes())),
                edits: registration_edits(config, edit, enabled),
            }
        }
        HookTarget::Rule { path } => {
            let body = format!(
                "---\ndescription: \"{name} — {}\"\nalwaysApply: true\n---\n\n{}",
                hook.description,
                hook.safety_prose()
            );
            Artifact::Registration {
                script: Some((placed(path), body.into_bytes())),
                edits: Vec::new(),
            }
        }
    }
}

/// Registering is always worth an edit; deregistering is only worth one when
/// the config file exists — bringing one into being to record an absence is a
/// change nobody asked for.
fn registration_edits(
    path: &std::path::Path,
    edit: ConfigEdit,
    enabled: bool,
) -> Vec<(PathBuf, ConfigEdit)> {
    if enabled || path.exists() {
        vec![(path.to_path_buf(), edit)]
    } else {
        Vec::new()
    }
}

pub(super) fn desired_mcp(ctx: &ItemCtx, state: &mut DesiredState) -> Result<()> {
    let text = ctx.sealed.read_to_string(ctx.item_path)?;
    let value = match mcp_value(&text) {
        Ok(value) => value,
        Err(problem) => {
            state.unreadable(
                ItemKind::McpServer,
                ctx.name,
                format!("mcp {}: {problem}", ctx.name),
            );
            return Ok(());
        }
    };
    for harness in ctx.harnesses.clone() {
        // Gemini splits the declaration from the record of whether it is on,
        // and the two live in different files at different scopes.
        let edits = if harness == HarnessId::Gemini {
            match super::gemini::mcp_edits(ctx, state, &value) {
                Some(edits) => edits,
                None => continue,
            }
        } else {
            let Some(registry) = mcp_registry(ctx.env, ctx.scope, harness) else {
                continue;
            };
            if harness == HarnessId::Copilot {
                super::copilot::switched_off_elsewhere(ctx, ItemKind::McpServer, state);
            }
            let edit = if ctx.decl.enabled {
                ConfigEdit::UpsertMcpServer {
                    name: ctx.name.to_owned(),
                    // Copilot names the transport on the entry itself, so a
                    // server written in another tool's shape would not load.
                    value: match harness {
                        HarnessId::Copilot => super::copilot::server(&value),
                        _ => value.clone(),
                    },
                }
            } else {
                ConfigEdit::RemoveMcpServer {
                    name: ctx.name.to_owned(),
                }
            };
            registration_edits(&registry, edit, ctx.decl.enabled)
        };
        let artifact = Artifact::Registration {
            script: None,
            edits,
        };
        state
            .items
            .push(declared(ctx, ItemKind::McpServer, harness, artifact)?);
    }
    Ok(())
}

/// `mcp/<name>.toml` → the JSON value claude stores under `mcpServers`.
/// Env values are `$`-references by contract: a literal is a secret in a
/// tracked file, so it is rejected rather than installed.
fn mcp_value(text: &str) -> std::result::Result<Value, String> {
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let string = |key: &str| {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let mut vars = Map::new();
    if let Some(env) = table.get("env").and_then(toml::Value::as_table) {
        for (key, value) in env {
            let reference = value.as_str().unwrap_or_default();
            if !reference.starts_with('$') {
                return Err(format!(
                    "env value for {key} must be a $REFERENCE, never a secret"
                ));
            }
            vars.insert(key.clone(), json!(reference));
        }
    }
    let transport = string("transport").unwrap_or_else(|| "stdio".to_owned());
    let mut server = Map::new();
    match transport.as_str() {
        "stdio" => {
            let command = string("command").ok_or("stdio transport needs a command")?;
            server.insert("command".into(), json!(command));
            if let Some(args) = table.get("args").and_then(toml::Value::as_array) {
                let args: Vec<Value> = args
                    .iter()
                    .filter_map(|a| a.as_str())
                    .map(|a| json!(a))
                    .collect();
                server.insert("args".into(), Value::Array(args));
            }
            if !vars.is_empty() {
                server.insert("env".into(), Value::Object(vars));
            }
        }
        "http" | "sse" => {
            let url = string("url").ok_or(format!("{transport} transport needs a url"))?;
            server.insert("type".into(), json!(transport));
            server.insert("url".into(), json!(url));
        }
        other => return Err(format!("unknown transport '{other}'")),
    }
    Ok(Value::Object(server))
}

/// Plugins carry no source content — the declaration is the whole item, and
/// applying it is one toggle in the settings file of the tool it names. Each
/// declaration names that tool: more than one harness reads an
/// `enabledPlugins` map, and a plugin installed for one of them is not a
/// plugin the others have.
pub(super) fn desired_plugins(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    state: &mut DesiredState,
) {
    for (key, decl) in &manifest.plugins {
        let harness = decl.harness;
        let toggle = crate::harness::capabilities(harness, ItemKind::Plugin).toggle;
        let supported = match scope {
            Scope::Global => toggle.global,
            Scope::Project { .. } => toggle.project,
        };
        let Some(settings) = plugin_settings(env, scope, harness).filter(|_| supported) else {
            state.notes.push(format!(
                "plugin {key}: {} has no plugin switch at this scope",
                harness.display_name()
            ));
            continue;
        };
        if harness == HarnessId::Copilot
            && let Some(reason) = super::copilot::plugin_refusal(env, scope)
        {
            state
                .notes
                .push(format!("plugin {key}: {reason} — nothing was switched"));
            continue;
        }
        state.items.push(Desired {
            key: entry_key(ItemKind::Plugin, key, harness),
            kind: ItemKind::Plugin,
            name: key.clone(),
            harness,
            enabled: decl.enabled,
            method: Method::Copy,
            source_name: "plugin".to_owned(),
            provenance: "marketplace".to_owned(),
            hash: hash_bytes(format!("plugin:{key}:{}", decl.enabled).as_bytes()),
            upstream_skills: None,
            emitted: None,
            artifact: Artifact::Registration {
                script: None,
                edits: vec![(
                    settings,
                    ConfigEdit::SetPluginEnabled {
                        key: key.clone(),
                        enabled: Some(decl.enabled),
                    },
                )],
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_transports_render_and_literal_env_values_are_refused() {
        let stdio = mcp_value(
            "command = \"gh-mcp\"\nargs = [\"--stdio\"]\n[env]\nGITHUB_TOKEN = \"$GH_TOKEN\"\n",
        )
        .unwrap();
        assert_eq!(stdio["command"], "gh-mcp");
        assert_eq!(stdio["args"][0], "--stdio");
        assert_eq!(stdio["env"]["GITHUB_TOKEN"], "$GH_TOKEN");

        let http = mcp_value("transport = \"http\"\nurl = \"https://mcp.example\"\n").unwrap();
        assert_eq!(http["type"], "http");
        assert_eq!(http["url"], "https://mcp.example");

        let secret = mcp_value("command = \"x\"\n[env]\nTOKEN = \"ghp_literal\"\n").unwrap_err();
        assert_eq!(
            secret,
            "env value for TOKEN must be a $REFERENCE, never a secret"
        );
        assert!(mcp_value("transport = \"stdio\"\n").is_err());
        assert!(mcp_value("transport = \"carrier-pigeon\"\n").is_err());
    }
}
