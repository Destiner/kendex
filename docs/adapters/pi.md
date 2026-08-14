# Pi

The only harness with an extension kind vstack installs end to end, and the
only one whose tool surface is deny-only over an open-ended vocabulary — an
allowlist there cannot be expressed and cannot be complemented without
widening, so it is refused rather than approximated.

## Roots

| Scope | Path | Relocated by |
|---|---|---|
| Global | `~/.pi/agent` | `PI_CODING_AGENT_DIR` |
| Project | `<project>/.pi`, plus the shared `<project>/.agents` | — |

Project markers: a `.pi/` or `.agents/` directory. Owner:
`crates/core/src/harness/pi.rs`.

## Surfaces

| Kind | Global | Project | Caps |
|---|---|---|---|
| agent | `~/.pi/agent/agents/*.md` | `.pi/agents/*.md` | managed, both |
| skill | `~/.pi/agent/skills/<name>/SKILL.md` | `.agents/skills/<name>/SKILL.md` — **shared with Codex** | managed, both |
| command | `~/.pi/agent/prompts/*.md` | `.pi/prompts/*.md` | observe only, both |
| hook | — | — | unsupported |
| mcp-server | — | — | unsupported |
| plugin | — | — | unsupported |
| pi-extension | `~/.pi/agent/settings.json` `packages[]`, and `~/.pi/agent/extensions/*.{ts,js}` | `.pi/settings.json` `packages[]`, and `.pi/extensions/*.{ts,js}` | managed, both |

Pi hooks belong to the `pi-hooks` extension, not to files vstack manages, so
the kind is unsupported rather than shimmed — and Pi's hook row is one of the
two that say nothing about enforcement, because there is no surface to
enforce anything on. Pi reads no MCP servers at all, which is why its
transport list is empty.

## Format facts

- **Byte cap:** none.
- **Name rule:** `Any`. Namespace separator `__`.
- **MCP transports:** none.
- **Agent file:** YAML frontmatter + markdown body, `<name>.md`. vstack
  writes `name`, `description`, `deny-tools?`, `allowed-subagents?`,
  `model?`, `color?` and `pane?` (`crates/core/src/render/agent/pi.rs`).
- **Model dialect:** `fable` and `opus` omit the key so the child inherits
  the parent session; other tiers resolve to `openai-codex/gpt-5.6-sol`. A
  bare id with no `/` passes through with a warning — Pi has no default
  provider to supply one. An effort setting rides along as a `:<effort>`
  suffix on the model id.
- **Frontmatter schema:** Pi reads plain markdown and enforces none, so the
  name rule is the whole of what the validator can check.

## Permissions

Deny-only. `allowed-subagents` and `deny-tools` have to agree, so they are
resolved together: engineers delegate to `scout` by default and every other
role is a leaf. `subagent`, `get_subagent_result`, `steer_subagent` and
`stop_subagent` are always denied; `delegate_subagent` is denied too unless
delegation was declared; everything but the planner denies `question`, and a
reviewer also denies `tasks_write`.

An `AllowOnly` intent is a hard refusal for this harness. Completing the
allowlist into a deny list would widen access the moment Pi grows a built-in
it never named, so nothing is rendered and the reason names both fixes: set
an explicit `deny-tools` override for Pi, or drop Pi from the agent's
harnesses.

## Pi extensions

An extension is an npm-shaped package. A source ships
`pi-extensions/<name>/`; vstack copies it into the scope's `packages/`
directory, resolves its production dependencies with npm
(`--omit=dev --package-lock=false --legacy-peer-deps --no-audit --no-fund`),
links its `bin` entries into the scope's `bin/`, registers it in the scope's
`settings.json`, and mirrors its `pi.appendSystem` file into the scope's
`APPEND_SYSTEM.md` as a marker block (`crates/core/src/pi_ext/`).

**Cross-scope duplicate guard.** Pi loads the global and project scopes
together and de-duplicates packages by identity, not by the resources they
register. The same package under two names, or at two scopes, registers twice
and crashes Pi at startup — so vstack checks for the duplicate before writing
(`duplicate_elsewhere`, `crates/core/src/pi_ext/renames.rs`).

## Migration and old-shape tolerance

The 1.0.0 release moved every catalog package under the `@vanillagreen/` npm
scope. Installs and locks predating that move still carry the old unscoped
names, and a few carry older names still (`pi-subagents`,
`prompt-stash`). The rename table maps each current name to every name it has
had, so an old install is recognized rather than reinstalled beside itself
(`RENAMES`, `crates/core/src/pi_ext/renames.rs`).
