# Changelog

Notable changes, per [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are written when a change lands, not batched at release. Breaking
changes carry a **Breaking** call-out with their migration note inline.

## [Unreleased]

### Changed

- **Breaking:** agent tool permissions are typed intent, preserved from
  source to every renderer and never widened. A missing `role:` no longer
  renders Codex `sandbox_mode = "danger-full-access"` (role-less agents get
  the sandbox their `tools:` list justifies); a source `tools:` allowlist
  renders natively on Claude, synthesizes an OpenCode permission block,
  infers the Codex sandbox, and is refused on Pi where honoring it is
  impossible; the v1 importer carries legacy `tools:` allowlists over as
  `allow-tools` overrides instead of dropping them. Migration: refresh
  regenerates installed agents; an agent that wants full access declares
  `role: engineer` explicitly.

- **Breaking:** model aliases resolve through one per-harness table
  (`fable`, `opus`, `sonnet`, `haiku`, `inherit`); `inherit` now survives
  every harness (OpenCode/Codex/Pi omit the field instead of emitting an
  invalid id such as `openai/inherit`), explicit vendor ids pass through
  untouched, and a bare unknown model warns where the harness's loader
  requires a `provider/model` form. Migration: refresh regenerates.

### Fixed

- Two or more settings changes to the same configuration file now apply
  together in one write: installing two MCP servers (or a hook plus a
  server, or any mix of registrations and removals) into one settings
  file in one apply used to fail and roll back; each file now gets a
  single composed mutation with a single precondition.

### Added

- Every catalog read goes through one sealed API: reads resolve against
  the canonical source root, symlinks in a catalog are refused loudly (a
  hostile catalog can no longer pull host files into generated artifacts
  or recurse forever), and traversal carries depth, count, and byte
  budgets. One refused item degrades to a note; the rest of the scope
  still plans.
- Source frontmatter is parsed as real YAML (block scalars, arrays, nested
  maps) with adversarial-input bounds: aliases, duplicate keys, oversized or
  deeply nested frontmatter are refused, and unknown keys warn instead of
  silently vanishing.

## [0.1.0] - 2026-08-10

First v2 release: desktop app (Tauri) + `vstack` CLI over one engine,
replacing vstack v1.

### Added

- Scan → declare → diff → apply engine over per-scope `vstack.toml`
  manifests, with preview-first, journaled, transactional applies and
  crash recovery; removals go to a trash, never a hard delete.
- Five harnesses — Claude Code, Codex, OpenCode, Cursor, Pi — behind one
  adapter seam with a single capability table gating every operation.
- Agents and skills authored once, rendered per tool; hooks, commands,
  MCP servers, plugins, and Pi extensions managed where each tool
  supports them.
- Catalog sources as plain git repos or local paths, enabled per scope;
  adopt brings hand-made files under management.
- CLI verbs mirroring every core operation: `add`, `remove`, `adopt`,
  `apply`, `refresh`, `verify`, `list`, `check`, `source`, `project`,
  `report`, `update`, `update-pi`, `import`, `init`.
- Self-updating app and CLI via a tag-driven draft-release feed.

### Breaking

- **Breaking:** fresh manifest and lock schema; v1 files are not read.
  Migration: `vstack import` converts v1 manifests and locks in place
  (originals copied to the trash first), then `vstack refresh`
  regenerates every installation.
- **Breaking:** v1 extras and theme packs are not carried over.
