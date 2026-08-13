use serde::{Deserialize, Serialize};
use specta::Type;

use crate::model::{HarnessId, ItemKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpSupport {
    pub project: bool,
    pub global: bool,
}

pub const BOTH: OpSupport = OpSupport {
    project: true,
    global: true,
};
pub const PROJECT: OpSupport = OpSupport {
    project: true,
    global: false,
};
pub const GLOBAL: OpSupport = OpSupport {
    project: false,
    global: true,
};
pub const NONE: OpSupport = OpSupport {
    project: false,
    global: false,
};

/// Whether the harness runs what vstack writes, or only reads it as prose.
/// `managed` says vstack can write and track an artifact; it never said the
/// tool would act on it, and a safety hook must not read as protection on a
/// tool that can merely suggest it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum Enforcement {
    /// The harness executes the command and honors its result.
    Enforced,
    /// Rendered as instructions the model is free to ignore.
    Advisory,
    /// Nothing here executes: every kind but Hook, and the harnesses vstack
    /// declares no hook surface for.
    NotApplicable,
}

/// What each operation supports for one harness × kind. `observe` is derived
/// from adapter surface declarations (tested); mutation columns land with
/// their phases and stay honest through those phases' tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KindCaps {
    pub observe: OpSupport,
    pub adopt: OpSupport,
    pub install: OpSupport,
    pub toggle: OpSupport,
    pub remove: OpSupport,
    pub refresh: OpSupport,
    /// The kind the harness actually stores this one as, when it has no
    /// surface of its own to write to. `observe` keeps describing the
    /// item's own surfaces; what a mutation writes is observable at the
    /// emitted kind's, which is where the honesty check looks.
    pub installs_as: Option<ItemKind>,
    /// Only Hook rows carry anything but `NotApplicable` — the axis exists
    /// to separate a hook the tool runs from one it merely reads.
    pub enforcement: Enforcement,
}

const fn unsupported() -> KindCaps {
    KindCaps {
        observe: NONE,
        adopt: NONE,
        install: NONE,
        toggle: NONE,
        remove: NONE,
        refresh: NONE,
        installs_as: None,
        enforcement: Enforcement::NotApplicable,
    }
}

/// Fully managed at the given scopes.
const fn managed(scopes: OpSupport) -> KindCaps {
    KindCaps {
        observe: scopes,
        adopt: scopes,
        install: scopes,
        toggle: scopes,
        remove: scopes,
        refresh: scopes,
        ..unsupported()
    }
}

/// Read-only surface: scanning works, nothing may be written.
const fn observe_only(scopes: OpSupport) -> KindCaps {
    KindCaps {
        observe: scopes,
        ..unsupported()
    }
}

/// A hook surface the harness itself runs.
const fn enforced(caps: KindCaps) -> KindCaps {
    KindCaps {
        enforcement: Enforcement::Enforced,
        ..caps
    }
}

/// A hook surface that renders to text the model may ignore.
const fn advisory(caps: KindCaps) -> KindCaps {
    KindCaps {
        enforcement: Enforcement::Advisory,
        ..caps
    }
}

/// The names a harness's loader can find an item under. Outside the rule,
/// the item is not merely untidy — the tool either skips it or lists it
/// under a spelling nobody typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameRule {
    /// Any name the manifest accepts, so long as it stays one path segment.
    Any,
    LowerKebab {
        /// `None` where the harness documents the spelling but no length.
        max_len: Option<usize>,
    },
}

/// How a harness reaches an MCP server. A server declared with a transport
/// its harness cannot speak does not load, so the renderers read this rather
/// than assuming every tool takes every form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    /// Streamable HTTP.
    Http,
    Sse,
}

use McpTransport::{Http, Sse, Stdio};

/// Format facts per harness — owned here beside the op table so renderers
/// and the surface model read one source of truth instead of scattering
/// literals. Extended axis by axis as consumers land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatCaps {
    /// Bytes the harness reliably loads from one SKILL.md before it
    /// truncates; `None` means no known cap. Oversized bodies split into
    /// `references/` rather than truncating.
    pub skill_body_max_bytes: Option<usize>,
    pub name_rule: NameRule,
    /// Empty where the harness reads no MCP servers at all.
    pub mcp_transports: &'static [McpTransport],
}

const fn format_defaults() -> FormatCaps {
    FormatCaps {
        skill_body_max_bytes: None,
        name_rule: NameRule::Any,
        mcp_transports: &[Stdio, Http, Sse],
    }
}

pub const fn format_caps(harness: HarnessId) -> FormatCaps {
    match harness {
        // Claude's three transports are the ones the MCP writer emits.
        HarnessId::Claude => format_defaults(),
        HarnessId::Codex => FormatCaps {
            skill_body_max_bytes: Some(8192),
            // Codex speaks Streamable HTTP, never SSE.
            mcp_transports: &[Stdio, Http],
            ..format_defaults()
        },
        // OpenCode keys agents and skills by a slug it will not coerce:
        // capitals and underscores make the item unloadable, not renamed.
        // Its servers are `local` (command) or `remote` (url) — no SSE
        // (opencode.ai/docs/mcp-servers).
        HarnessId::Opencode => FormatCaps {
            name_rule: NameRule::LowerKebab { max_len: Some(64) },
            mcp_transports: &[Stdio, Http],
            ..format_defaults()
        },
        // Cursor takes a command, an SSE url, or a streamable-HTTP url
        // (cursor.com/docs/context/mcp).
        HarnessId::Cursor => format_defaults(),
        // pi reads no MCP servers.
        HarnessId::Pi => FormatCaps {
            mcp_transports: &[],
            ..format_defaults()
        },
        // Gemini server entries carry `command`, `httpUrl`, or `url`, and its
        // skills take any name (matrix §1).
        HarnessId::Gemini => format_defaults(),
        // Copilot's `type` is local|stdio|http|sse, and a SKILL.md `name` is
        // required in lowercase-hyphen with no documented length (matrix §2).
        HarnessId::Copilot => FormatCaps {
            name_rule: NameRule::LowerKebab { max_len: None },
            ..format_defaults()
        },
    }
}

/// Whether any kind installs on this harness at any scope. A tool vstack
/// only reads is not an install target: naming it in a manifest or an
/// editor picker promises writes that never happen.
pub fn installable(harness: HarnessId) -> bool {
    ItemKind::ALL.into_iter().any(|kind| {
        let install = capabilities(harness, kind).install;
        install.project || install.global
    })
}

pub fn capabilities(harness: HarnessId, kind: ItemKind) -> KindCaps {
    use HarnessId::*;
    use ItemKind::*;
    match (harness, kind) {
        (Claude, Agent | Skill | Command) => managed(BOTH),
        // Claude runs the registered command and gates the tool call on its
        // exit status.
        (Claude, Hook) => enforced(managed(BOTH)),
        (Claude, McpServer) => managed(BOTH),
        // Plugin install/remove is parked with the marketplace work.
        (Claude, Plugin) => KindCaps {
            observe: BOTH,
            toggle: BOTH,
            ..observe_only(BOTH)
        },
        (Claude, PiExtension) => unsupported(),

        (Codex, Agent | Skill) => managed(BOTH),
        // Native for the events `hook.rs` maps onto codex's own; anything
        // outside that map renders as prose the model may ignore.
        (Codex, Hook) => enforced(managed(BOTH)),
        // Codex deprecated `~/.codex/prompts` in favor of skills, so a
        // command installs as a skill and is read back from the skill
        // surface. The prompts dir still loads, so it is still scanned —
        // and never written, which is why adopt stays off.
        (Codex, Command) => KindCaps {
            observe: GLOBAL,
            install: BOTH,
            toggle: BOTH,
            remove: BOTH,
            refresh: BOTH,
            installs_as: Some(Skill),
            ..unsupported()
        },
        (Codex, McpServer) => observe_only(BOTH),
        (Codex, Plugin) => observe_only(GLOBAL),
        (Codex, PiExtension) => unsupported(),

        (Opencode, Agent | Skill) => managed(BOTH),
        // Hooks render as instruction files + config refs; only those managed
        // artifacts are observable — opencode has no native hook surface, so
        // nothing here runs.
        (Opencode, Hook) => advisory(managed(BOTH)),
        (Opencode, Command) => observe_only(BOTH),
        (Opencode, McpServer) => observe_only(BOTH),
        (Opencode, Plugin) => observe_only(BOTH),
        (Opencode, PiExtension) => unsupported(),

        // Cursor is managed project-only (no global agent scope in v1), but
        // its global command/MCP surfaces exist and are observed.
        (Cursor, Agent) => managed(PROJECT),
        // Skills share the rules dir and cannot be told apart from agents.
        (Cursor, Skill) => unsupported(),
        // A cursor hook is a `.mdc` rule with no registration behind it.
        (Cursor, Hook) => advisory(KindCaps {
            observe: BOTH,
            ..managed(PROJECT)
        }),
        (Cursor, Command) => observe_only(BOTH),
        (Cursor, McpServer) => observe_only(BOTH),
        (Cursor, Plugin) => observe_only(GLOBAL),
        (Cursor, PiExtension) => unsupported(),

        (Pi, Agent | Skill) => managed(BOTH),
        // pi hooks belong to the pi-hooks extension, not to files we manage.
        (Pi, Hook) => unsupported(),
        (Pi, Command) => observe_only(BOTH),
        (Pi, McpServer) => unsupported(),
        (Pi, Plugin) => unsupported(),
        (Pi, PiExtension) => managed(BOTH),

        // Both scopes hold the same file layout under their own root, and
        // only `.toml` loads from the commands dir, so the rename toggle is
        // safe there (matrix §1, §7).
        (Gemini, Agent | Skill | Command) => managed(BOTH),
        // Gemini runs hooks from the `hooks` key of settings.json at either
        // scope — 11 events, regex matchers, exit codes honored (matrix §D1).
        (Gemini, Hook) => enforced(managed(BOTH)),
        // A server is declared per scope, but the file recording whether it
        // is on is a single global one, so switching one off is a global
        // act; doing it under a project lock would write outside the scope
        // that holds the lock (matrix §1, plan Phase 2).
        (Gemini, McpServer) => KindCaps {
            toggle: GLOBAL,
            ..managed(BOTH)
        },
        // Extensions install globally only, and their enablement is an
        // undocumented path-rule file, so nothing here is written (§R1).
        (Gemini, Plugin) => observe_only(GLOBAL),
        (Gemini, PiExtension) => unsupported(),

        // A skill or server switches by the rename and the entry vstack owns,
        // both ways. Copilot's own `disabledSkills` list is a separate hold
        // a repository cannot lift, and the audit says so per item rather
        // than the table claiming vstack's switch is one-way (matrix §R7).
        (Copilot, Agent | Skill | McpServer) => managed(BOTH),
        // No file-backed slash-command surface exists in any Copilot product
        // — prompt files are IDE-only (matrix §D8).
        (Copilot, Command) => unsupported(),
        // Copilot runs hooks from `.github/hooks/*.json` and
        // `~/.copilot/hooks/*.json` at 14 events, honoring the exit code
        // (matrix §D9). Entries inline in a settings file are read and never
        // written: they have no switch of their own to flip (§R5).
        (Copilot, Hook) => enforced(managed(BOTH)),
        // The `enabledPlugins` map is a clean boolean flip at either scope.
        // Installing one needs marketplace resolution, which is parked with
        // the Claude marketplace work (matrix §7).
        (Copilot, Plugin) => KindCaps {
            observe: BOTH,
            toggle: BOTH,
            ..observe_only(BOTH)
        },
        (Copilot, PiExtension) => unsupported(),
    }
}
