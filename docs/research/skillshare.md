# skillshare — provenance, ownership, uninstall, grouping

`runkids/skillshare` @ `14959e2` (v0.20.25), Go. Scoped survey. Supersedes the
deleted `report-skillshare.md` twice over: the per-skill sidecar is now
deprecated, and uninstall (unread there) is documented below.

## Provenance — two layers, deliberately separate

| Layer | File | Lives in | Records |
|---|---|---|---|
| Where an item came from | `.metadata.json` | the *source* dir (`skills/`, `agents/`) | one entry per item |
| What we own in a target | `.skillshare-manifest.json` | each *target* dir (`.claude/skills/`, …) | `flatName → "symlink"` or source SHA-256, plus source mtimes |

`MetadataStore {version, entries: map[name]*MetadataEntry}`,
`internal/install/metadata.go:13-47`; entry fields `Source`, `Kind`, `Type`,
`Tracked`, `Group`, `Branch`, `Into`, `InstalledAt`, `RepoURL`, `Subdir`,
`Version` (commit), `TreeHash`, `FileHashes` (per-file `sha256:<hex>`).
Per-skill `.skillshare-meta.json` sidecars (`meta.go:14`) are **deprecated** in
favor of it (`metadata_migrate.go` folds old ones in) — state moved from
scattered-in-the-tree to one file per directory, still inside the managed tree.
Reinstall conflicts compare stored `RepoURL` to the incoming source: same repo →
friendly no-op, different repo → hard error naming the original owner, unknown
origin → `--force` (`internal/install/install.go`, `checkExistingConflict`).

## Ownership on sync

Sync acts only on entries listed in the target's manifest (`internal/sync/copy.go:39-236`):

| Target state | Decision |
|---|---|
| in manifest, source checksum unchanged | skip (`copy.go:171-178`) |
| in manifest, source changed | `RemoveAll` + re-copy, record new checksum (`copy.go:180-200`) |
| not in manifest (user-local) | preserve; `--force` needed to touch it (`copy.go:166-168`) |
| in manifest, gone from source | prune via hard `os.RemoveAll` — not trash (`copy.go:268-289`) |
| not in manifest, gone from source | never considered; pruning iterates `manifest.Managed` only |

Change detection is two-tier: source `DirMaxMtime` fast path falling back to a
deterministic SHA-256 over sorted path+content pairs (`copy.go:112`).
**User edits to installed copies are not tracked** — the comparison is
recorded-source-checksum vs current-source-checksum and the target's bytes are
never hashed, so an edit survives silently while the source is unchanged and
dies without warning the moment it isn't. `FileHashes` could answer this and is
not consulted on sync.

## Uninstall

- **Trash, not delete** — `MoveToTrash`, 7-day retention (`internal/trash/trash.go:17,192`),
  `Restore` at `:352`, expired entries cleaned opportunistically each run
  (`cmd/skillshare/uninstall.go:497,520,539`). Separate trash roots per scope and kind.
- **Dirty-repo guard** — git-tracked installs get parallel `git.IsDirty` checks and a refusal:
  *"uncommitted changes detected, use --force to override"* (`uninstall.go:796-861`). The only
  user-edit protection, and only because git supplies it; plain copies get just the 7-day net.
- **Metadata swept after the files** — exact path matches, then entries whose `Group` matches a
  removed repo dir, then entries name-prefixed by one (`handler_uninstall.go:278-310`); matching
  `.gitignore` lines stripped too.
- No dependency concept, so no dependency-aware uninstall.

## Grouping — directories, not bundles

`Group` is a subdirectory name set by `--into` at install (`schemas/registry.schema.json:47-49`,
`install_config.go:313`), nesting as `parent/child` (`metadata_migrate.go:170-172`); a
tracked-repo install creates one implicitly, named after the repo (`handler_skills.go:417`).
Groups are addressable as a unit by repeatable `--group/-G` across `update`, `check`, `audit`,
`uninstall` (`update.go:93`, `check.go:116`, `uninstall_project.go:148-158`). There is no member
list and no identity apart from the directory: "removing a bundle removes its members" holds
only trivially, because the group *is* the directory.

## What vstack should copy or avoid

Copy the two-layer split — provenance keyed to the source, kept separate from a per-target
manifest of what we own there. It is vstack2's Item vs Installation distinction, and what makes
"prune only what we put here, warn about the rest" safe rather than destructive. Copy the
trash-with-retention default and the provenance-compare on reinstall (same source → no-op,
different source → error naming the owner); invariants 4, 6, and 7 already commit to both.
Avoid three things. State inside the managed tree: both files sit where a user can gitignore,
edit, or delete them, and a corrupt manifest is silently treated as empty (`manifest.go:34-36`),
turning a parse bug into "we own nothing" and orphaning every managed file. The sync blind spot:
source hashes decide overwrites while installed bytes are never checked, so local edits die
unannounced — invariant 6 demands the target-side comparison skillshare skips, and the
`FileHashes` it already stores prove the data is cheap to have. And do not model bundles as
directories: `Group` gets unit operations for free but cannot express a curated set spanning
kinds or sources, cannot be versioned, and cannot survive a member moving.
