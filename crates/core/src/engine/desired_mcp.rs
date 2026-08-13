//! One declared MCP server as each tool records it: the registry file it
//! belongs in, the shape that tool keys an entry by, and the `mcp/<name>.toml`
//! the catalog ships it as.

use serde_json::{Map, Value, json};

use super::desired::{Artifact, DesiredState, ItemCtx};
use super::desired_kinds::{declared, registration_edits};
use super::targets::mcp_registry;
use crate::configedit::ConfigEdit;
use crate::error::Result;
use crate::model::{HarnessId, ItemKind};

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
