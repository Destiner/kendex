//! What the shared declaration paths have to ask Gemini before they write.
//! Two facts drive everything here: Gemini's settings file gained a nested
//! schema that older CLIs do not read, and a system settings layer sits
//! above both the user's and the project's (matrix §R2, §R9).

use std::path::PathBuf;

use serde_json::Value;

use super::ItemWarning;
use super::desired::{DesiredState, ItemCtx};
use crate::configedit::ConfigEdit;
use crate::harness::gemini::settings::{
    Settings, mcp_enablement_file, read, settings_file, system_defines, system_settings_file,
};
use crate::hook::HookSource;
use crate::model::{HarnessId, ItemKind, Scope};

fn settings(ctx: &ItemCtx) -> Settings {
    read(&settings_file(ctx.env, ctx.scope))
}

/// The system settings layer outranks both the user's file and the
/// project's, so a key it sets can leave what vstack writes inert. Only what
/// is on disk is observable, so the wording says how things are configured
/// and never claims what a run will do (matrix §R2).
fn overridden(ctx: &ItemCtx, kind: ItemKind, key: &str) -> Option<ItemWarning> {
    system_defines(ctx.env, key).then(|| ItemWarning {
        kind,
        name: ctx.name.to_owned(),
        harness: Some(HarnessId::Gemini),
        message: format!(
            "this machine's system-wide Gemini settings also set `{key}`, which outranks both your settings and this project — as configured, what vstack writes here can be overridden"
        ),
        remediation: Some(format!(
            "ask whoever manages {} to make room for it, or install this at a scope that file leaves alone",
            system_settings_file(ctx.env).display()
        )),
    })
}

/// What the machine's own configuration says about an agent vstack is about
/// to write. An installation that cannot run must not read as one that can.
pub(super) fn agent_notices(ctx: &ItemCtx, state: &mut DesiredState) {
    // Gemini's own default for this flag is on, so only an explicit `false`
    // means the feature is off — absence is not the feature being missing
    // (configuration reference; matrix §1, §R3).
    if settings(ctx).agents_enabled == Some(false) {
        state.warnings.push(ItemWarning {
            kind: ItemKind::Agent,
            name: ctx.name.to_owned(),
            harness: Some(HarnessId::Gemini),
            message:
                "Gemini's subagents are switched off in its settings, so this agent installs but stays inert"
                    .to_owned(),
            remediation: Some(
                "turn `experimental.enableAgents` on in Gemini's settings, or drop Gemini from this agent's harnesses"
                    .to_owned(),
            ),
        });
    }
    state
        .warnings
        .extend(overridden(ctx, ItemKind::Agent, "agents"));
}

/// The hook as Gemini would register it, or `None` with the note saying why
/// nothing is registered: an event Gemini has no counterpart for, or a
/// settings file the installed CLI would not read back.
pub(super) fn hook(
    ctx: &ItemCtx,
    hook: &HookSource,
    state: &mut DesiredState,
) -> Option<HookSource> {
    if let Some(reason) = settings(ctx).unmanageable() {
        state.notes.push(format!(
            "hook {}: {reason} — nothing was registered for Gemini",
            ctx.name
        ));
        return None;
    }
    let Some(translated) = crate::harness::gemini::hook_for(hook) else {
        state.notes.push(format!(
            "hook {}: event {} has no Gemini counterpart, and hanging it on a near-miss would run it at the wrong moment",
            ctx.name, hook.event
        ));
        return None;
    };
    state
        .warnings
        .extend(overridden(ctx, ItemKind::Hook, "hooks"));
    Some(translated)
}

/// The server entry as Gemini keys it: a streamable-HTTP endpoint is
/// `httpUrl` and an SSE one is plain `url`, with no `type` beside either
/// (matrix §1). Written in the shape another tool uses, an HTTP server
/// would load as SSE and reach nothing.
fn server(value: &Value) -> Value {
    let Some(url) = value.get("url").and_then(Value::as_str) else {
        return value.clone();
    };
    let key = match value.get("type").and_then(Value::as_str) {
        Some("http") => "httpUrl",
        _ => "url",
    };
    let mut entry = value.clone();
    let Some(object) = entry.as_object_mut() else {
        return entry;
    };
    object.remove("type");
    object.remove("url");
    object.insert(key.to_owned(), Value::String(url.to_owned()));
    entry
}

/// Every edit one declared MCP server takes on Gemini, or `None` when the
/// declaration cannot be honored here. The server itself is declared in the
/// settings file for its scope; whether it is switched on is recorded in one
/// global file, whatever scope declared it (matrix §1).
pub(super) fn mcp_edits(
    ctx: &ItemCtx,
    state: &mut DesiredState,
    value: &Value,
) -> Option<Vec<(PathBuf, ConfigEdit)>> {
    if let Some(reason) = settings(ctx).unmanageable() {
        state.notes.push(format!(
            "mcp {}: {reason} — nothing was declared for Gemini",
            ctx.name
        ));
        return None;
    }
    if !ctx.decl.enabled && matches!(ctx.scope, Scope::Project { .. }) {
        state.notes.push(format!(
            "mcp {}: Gemini records whether a server is on in one file for the whole machine, so a project can declare one but not switch it off — remove it here instead",
            ctx.name
        ));
        return None;
    }
    let enablement = mcp_enablement_file(ctx.env);
    let mut edits = vec![(
        settings_file(ctx.env, ctx.scope),
        ConfigEdit::UpsertMcpServer {
            name: ctx.name.to_owned(),
            value: server(value),
        },
    )];
    // Switching one off keeps the declaration and records the state; turning
    // one back on drops our record so Gemini's own default applies again —
    // and only when there is already a file holding it.
    match (ctx.decl.enabled, enablement.exists()) {
        (true, false) => {}
        (enabled, _) => edits.push((
            enablement,
            ConfigEdit::SetGeminiMcpEnabled {
                name: ctx.name.to_owned(),
                enabled: match enabled {
                    true => None,
                    false => Some(false),
                },
            },
        )),
    }
    state
        .warnings
        .extend(overridden(ctx, ItemKind::McpServer, "mcpServers"));
    Some(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_streamable_http_server_is_keyed_apart_from_an_sse_one() {
        assert_eq!(
            server(&json!({"type": "http", "url": "https://mcp.example"})),
            json!({"httpUrl": "https://mcp.example"})
        );
        assert_eq!(
            server(&json!({"type": "sse", "url": "https://mcp.example"})),
            json!({"url": "https://mcp.example"})
        );
        let stdio = json!({"command": "gh-mcp", "args": ["--stdio"]});
        assert_eq!(server(&stdio), stdio);
    }
}
