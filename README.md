# vstack

One place to manage AI coding-harness customizations — agents, skills,
hooks, commands, MCP servers, plugins, and Pi extensions — across Claude
Code, Codex, opencode, Cursor, and Pi, at global and per-project scope.

Desktop app + CLI over one engine. Four verbs, one model:

```
scan → declare → diff → apply
```

- **Scan** reads harness-native directories in place. Useful with zero
  adoption; nothing is copied into a shadow store.
- **Declare** — a per-scope `vstack.toml` is the only durable home of your
  intent.
- **Diff** — drift is declared vs. observed; the app's Sync page is that
  diff.
- **Apply** — make disk match declaration, plan shown first. Every apply is
  transactional: pre-images are journaled, failures roll back, interrupted
  applies recover on next run, and removals go to a trash — never straight
  to delete.

## Install

Build from source (Rust + Node required):

```sh
cargo build --release -p vstack-cli               # the `vstack` CLI
npm ci --prefix ui
cd crates/app && ../../ui/node_modules/.bin/tauri dev   # the desktop app
```

## Quick start

```sh
vstack owner/catalog-repo --agent rust --skill github   # declare + install
vstack list                                             # what exists, everywhere
vstack verify                                           # non-zero exit on drift
vstack refresh                                          # regenerate from sources
vstack adopt skill handmade                             # bring an unmanaged item under management
vstack apply --plan                                     # preview the full reconcile
```

Coming from v1: `vstack import` migrates manifests and locks in place
(originals are copied to the trash first), then `vstack refresh`
regenerates everything.

## The rules the engine keeps

1. Generated artifacts are always overwritable; your intent lives only in
   `vstack.toml`.
2. Nothing you set is ever clobbered, and nothing you removed is ever
   silently re-added.
3. Unmanaged files are reported, never touched. Foreign symlinks are
   conflicts, not clobber targets.
4. An item's recorded source is durable — a name collision across sources
   is a hard error naming the original.
5. Enable/disable is a lossless rename or a structured config edit that
   preserves every unrelated key.
6. One writer per scope: concurrent applies get a clear "busy" error,
   never an interleaved write.

## CLI surface

| Verb | Does |
|---|---|
| `add` (or bare `vstack <source>`) | declare + install agents/skills/… from a source |
| `remove`, `adopt`, `apply` | undeclare, take ownership, reconcile |
| `refresh` | re-resolve sources, regenerate every installation |
| `verify` | drift check; exit 1 on any failing row |
| `list` (`ls`), `check` | observe everything; sanity report |
| `source add/remove/enable/disable/refresh` | manage catalogs per scope |
| `project add/remove/list/discover` | the app's project registry |
| `report` | file an issue, routed to the asset's owner |
| `update`, `update-pi`, `import`, `init` | self-update, Pi packages, v1 migration, catalog scaffolding |

Scopes: `--scope project|global|all` (v1-compatible aliases `p/local`,
`g/user`, `both/*`), `-g` as a shortcut for global.
