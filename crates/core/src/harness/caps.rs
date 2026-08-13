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
}

const fn unsupported() -> KindCaps {
    KindCaps {
        observe: NONE,
        adopt: NONE,
        install: NONE,
        toggle: NONE,
        remove: NONE,
        refresh: NONE,
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
    }
}

/// Read-only surface: scanning works, nothing may be written.
const fn observe_only(scopes: OpSupport) -> KindCaps {
    KindCaps {
        observe: scopes,
        adopt: NONE,
        install: NONE,
        toggle: NONE,
        remove: NONE,
        refresh: NONE,
    }
}

/// Format facts per harness — owned here beside the op table so renderers
/// and the surface model read one source of truth instead of scattering
/// literals. Extended axis by axis as consumers land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatCaps {
    /// Bytes the harness reliably loads from one SKILL.md before it
    /// truncates; `None` means no known cap. Oversized bodies split into
    /// `references/` rather than truncating.
    pub skill_body_max_bytes: Option<usize>,
}

pub const fn format_caps(harness: HarnessId) -> FormatCaps {
    match harness {
        HarnessId::Codex => FormatCaps {
            skill_body_max_bytes: Some(8192),
        },
        _ => FormatCaps {
            skill_body_max_bytes: None,
        },
    }
}

pub fn capabilities(harness: HarnessId, kind: ItemKind) -> KindCaps {
    use HarnessId::*;
    use ItemKind::*;
    match (harness, kind) {
        (Claude, Agent | Skill | Hook | Command) => managed(BOTH),
        (Claude, McpServer) => managed(BOTH),
        // Plugin install/remove is parked with the marketplace work.
        (Claude, Plugin) => KindCaps {
            observe: BOTH,
            toggle: BOTH,
            ..observe_only(BOTH)
        },
        (Claude, PiExtension) => unsupported(),

        (Codex, Agent | Skill | Hook) => managed(BOTH),
        // `~/.codex/prompts` is a deprecated-but-loading surface: never write.
        (Codex, Command) => observe_only(GLOBAL),
        (Codex, McpServer) => observe_only(BOTH),
        (Codex, Plugin) => observe_only(GLOBAL),
        (Codex, PiExtension) => unsupported(),

        (Opencode, Agent | Skill) => managed(BOTH),
        // Hooks render as instruction files + config refs; only those managed
        // artifacts are observable — opencode has no native hook surface.
        (Opencode, Hook) => managed(BOTH),
        (Opencode, Command) => observe_only(BOTH),
        (Opencode, McpServer) => observe_only(BOTH),
        (Opencode, Plugin) => observe_only(BOTH),
        (Opencode, PiExtension) => unsupported(),

        // Cursor is managed project-only (no global agent scope in v1), but
        // its global command/MCP surfaces exist and are observed.
        (Cursor, Agent) => managed(PROJECT),
        // Skills share the rules dir and cannot be told apart from agents.
        (Cursor, Skill) => unsupported(),
        (Cursor, Hook) => KindCaps {
            observe: BOTH,
            ..managed(PROJECT)
        },
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
    }
}
