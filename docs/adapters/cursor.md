# Cursor

The narrowest adapter. Cursor has no agents and no skills — it has rules —
so an agent installs as a rule file, skills are unsupported rather than
misreported, and nothing about a rule is enforced.

## Roots

| Scope | Path | Relocated by |
|---|---|---|
| Global | `~/.cursor` | nothing |
| Project | `<project>/.cursor` | — |

Project marker: a `.cursor/` directory. Owner:
`crates/core/src/harness/cursor.rs`.

## Surfaces

| Kind | Global | Project | Caps |
|---|---|---|---|
| agent | — no global rules dir | `.cursor/rules/*.mdc` | managed, project only |
| skill | — | — | unsupported |
| hook | `~/.cursor/hooks.json` | `.cursor/hooks.json` | observe both; install/toggle/remove/refresh project only; **advisory** |
| command | `~/.cursor/commands/*.md` | `.cursor/commands/*.md` | observe only, both |
| mcp-server | `~/.cursor/mcp.json` | `.cursor/mcp.json` | observe only, both |
| plugin | `~/.cursor/plugins/{local,cache}` with `.cursor-plugin/plugin.json` | — | observe only, global |
| pi-extension | — | — | unsupported |

**Skills are unsupported on purpose.** They share the rules directory with
agents and cannot be told apart from them, so vstack declines to guess rather
than reporting one as the other.

**Cursor is managed project-only.** There is no global rules directory, so
the global agent surface is empty; the global command, MCP and plugin
surfaces do exist and are scanned.

## Format facts

- **Byte cap:** none.
- **Name rule:** `Any`. Namespace separator `__`.
- **MCP transports:** stdio, streamable HTTP, SSE — a command, an SSE url or
  a streamable-HTTP url ([cursor.com/docs/context/mcp](https://cursor.com/docs/context/mcp)).
- **Rule file:** `<name>.mdc`, YAML frontmatter + markdown. vstack writes
  exactly `description` (the agent's name and description joined) and
  `alwaysApply: false`. Rules carry no model, tool, skill or hook fields, so
  only the prompt survives (`crates/core/src/render/agent/cursor.rs`).
- **Model dialect:** every tier resolves to nothing — the renderer drops the
  field, because rules have none.
- **Frontmatter keys Cursor honors:** `description`, `globs`, `alwaysApply`.
  Anything else is advisory folklore, and the validator says so with that
  word (`CURSOR_KEYS`, `crates/core/src/render/validate/agent.rs`).

## Permissions

A rule grants no tools, so a permission intent cannot be widened here — but
it cannot be enforced either. Any intent other than `Unspecified` produces a
warning saying the restriction is advisory text only, with the fix being to
drop Cursor from that agent's harnesses if the restriction must hold.

## Hooks

**Advisory, and the artifact is a rule, not a registration.** A Cursor hook
is a `.mdc` file at `.cursor/rules/safety-<name>.mdc` carrying the hook's
description and its safety prose with `alwaysApply: true`, and there is no
registration behind it (`HookTarget::Rule`,
`crates/core/src/engine/targets.rs`).

Note the asymmetry: `hooks.json` is the surface vstack *observes* at both
scopes, while what it *writes* is a rule in the rules directory. Cursor's
own `hooks.json` is read and never written, and the global scope has no hook
target at all — a hook declared for Cursor at global scope installs nothing.

Disabling renames the rule file to `.disabled`.
