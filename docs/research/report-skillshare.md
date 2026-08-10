## Skillshare (runkids/skillshare) — Analysis Report

### 1. What it is

A single-binary Go CLI (`skillshare`) that manages skills, agents, and "extras" (rules/commands/prompts) for 60+ AI CLI tools (Claude Code, Cursor, Codex, OpenClaw, Gemini, etc.) from one canonical source directory, syncing them out to each tool's own directory via symlinks (or copies). Also ships a web UI (`skillshare ui`), TUI, and a security-audit engine. All paths below are relative to a clone of https://github.com/runkids/skillshare at commit `14959e2` (see PLAN.md References for re-clone instructions).

### 2. Modeling, storage, sync, sharing

**Directory layout (global mode)** — `internal/config/config.go:453-469`:
- `BaseDir()` = `$XDG_CONFIG_HOME/skillshare` → `%AppData%\skillshare` (Windows) → `~/.config/skillshare`.
- Canonical source dirs default to `<BaseDir>/skills/`, `<BaseDir>/agents/`, and extras live in a sibling dir derived from the skills source (`config.go:255-266, 303-340`). All three can be overridden independently via a `sources:` block (`Sources.Skills/Agents/Extras`), added in v0.19.16 to replace legacy flat `source:`/`agents_source:`/`extras_source:` keys — old keys still parsed for back-compat.
- Config file: `<BaseDir>/config.yaml` (`ConfigPath()`, overridable via `SKILLSHARE_CONFIG` env var). Schema-annotated YAML (`# yaml-language-server: $schema=...`) pointing at a public JSON Schema in `schemas/`.

**Project (repo) scope** — `internal/config/project.go`:
- `.skillshare/config.yaml` in the repo root (`ProjectConfigPath`), sources default to `.skillshare/skills/`, `.skillshare/agents/`, `.skillshare/extras/`. Project skills are meant to be committed with the repo (team-shared, no user-global side effects). Confirmed in the repo's own dogfood config at `.skillshare/config.yaml`:
  ```yaml
  targets:
      - universal
      - claude
  audit:
      block_threshold: CRITICAL
  ```

**Registry** — `internal/config/registry.go`: a separate `registry.yaml` (not `config.yaml`) tracks installed/remote skills (`SkillEntry{Name, Kind, Source, Tracked, Group, Branch}`). Lives at the *source root* (walks up to find `.git`, `SourceRoot()`), not the config dir — so registry.yaml is itself git-trackable alongside the skills it describes. One-time migrations exist for both "config.yaml embedded skills[] → registry.yaml" (`migrateSkillsToRegistry`) and "registry.yaml moved from config dir to source dir" (`MigrateRegistryToSource`), each guarded and logged, never silent-overwriting.

**Target registry** — `internal/config/targets.go` + embedded `internal/config/targets.yaml` (66 built-in tools). Each target spec declares global/project skill paths, optional agent paths, `also_scans` (extra paths a tool's runtime independently scans — used by `doctor` to warn about cross-target discovery overlap, e.g. Codex scanning `~/.agents/skills` even though its primary path is `~/.codex/skills`), and `aliases`. This is a clean, data-driven way to support many CLIs without hardcoding logic per tool.

**Scoping mechanisms** (multiple layers, composable):
- Global vs. project (separate config files/sources as above).
- Per-target `include`/`exclude` glob filters on skill flat-names (`ResourceTargetConfig`, `internal/sync/filter.go`).
- Per-skill `targets:` frontmatter field in SKILL.md — skills can declare which targets they apply to; skills with no `targets` field sync everywhere (`FilterSkillsByTarget`, `internal/sync/filter.go:125-138`; `config.MatchesTargetName` handles alias resolution).
- `.skillignore` / `.skillignore.local` (gitignore-syntax, per-repo, supports negation, `**`, anchoring) — `internal/skillignore/skillignore.go`. A parallel `.agentignore` exists for agents.
- `target_naming`: `flat` (default, `_group__nested__name` joined with `__`) vs `standard` (uses the skill's own frontmatter `name`, validated against strict rules — lowercase/digits/hyphens only, ≤64 chars, must equal dir name) — `internal/sync/target_naming.go`.

**Conflict handling**:
- Duplicate skill names under `standard` naming are detected and *excluded* from that target's sync rather than silently overwriting one (`CheckNameCollisionsForTargets`, `ResolveTargetSkillsForTarget` in `target_naming.go:79-104`), with warnings surfaced through `DiagOutput`.
- Install-time conflicts: `checkExistingConflict` (`internal/install/install.go:121-159`) compares the existing skill's stored `repo_url` metadata against the incoming source — same-repo reinstall becomes a friendly no-op (`ErrSkipSameRepo`, "use update or --force"), different-repo conflict is a hard error naming the original provenance, and empty/unknown-origin directories require `--force`. URL comparison normalizes `.git` suffix, protocol, and `user@host:` vs `https://` forms (`repoURLsMatch`/`normalizeCloneURL`).
- Update rollback safety: `updateTrackedRepo` (`internal/install/install_tracked.go:135-182`) records the pre-pull commit hash and aborts *before* pulling if it can't be determined; a failed post-pull audit rolls back via `git reset` rather than deleting the repo.

### 3. Install / uninstall mechanics

**Link vs copy** — `internal/sync/sync.go`, `internal/utils/link.go`:
- Modes: `symlink` (whole target dir is one symlink to source), `merge` (default — each skill gets its own symlink inside the target dir, letting user-local skills coexist), `copy` (each skill is a real copy, tracked via manifest+checksum for update detection).
- Cross-platform: real symlinks on macOS/Linux, NTFS junctions on Windows (no admin rights required) — `utils.IsSymlinkOrJunction` abstracts over both (`link.go`, `link_unix.go`, Windows variant not read but referenced).
- Relative vs absolute symlinks: when both source and target live under the same project root, links are created relative (`shouldUseRelative`) so a cloned repo remains portable; otherwise absolute.
- State machine per target: `StatusNotExist → CreateSymlink`, `StatusHasFiles → MigrateToSource` (moves/merges pre-existing local content into the source dir, then links it — so switching a target into skillshare-management doesn't lose data), `StatusConflict` (symlink pointing elsewhere — hard error, doesn't clobber), `StatusBroken → recreate` (`sync.go:157-306`).
- **Copy mode change detection**: two-tier — a cheap mtime fast-path (`DirMaxMtimeWithIgnore`, skips checksum if source's newest mtime is unchanged and target dir still exists) falling back to a deterministic SHA-256 over sorted relative-path+content pairs (`DirChecksumWithIgnore`, `internal/sync/copy.go:356-436`). Manifest (`.skillshare-manifest.json`) records `flatName → checksum-or-"symlink"` plus mtimes, distinguishing skillshare-managed entries from user-local ones so pruning never touches local content.
- **Pruning**: `PruneOrphanLinks`/`PruneOrphanCopies` remove target entries no longer present upstream, but only entries recognized as skillshare-managed (via manifest or, for legacy flat naming, a directory-name heuristic); unrecognized local directories are reported as warnings, never deleted, unless `--force`.

**Install from remote** — `internal/install/source.go`, `install_tracked.go`:
- URL parser (`ParseSourceWithOptions`) supports GitHub shorthand (`owner/repo`), GitHub/GitLab/Bitbucket/Azure DevOps (cloud + on-prem) HTTPS/SSH URLs, `file://`, GitHub `tree/blob` web-URL forms (auto-stripped), and GitLab `-/tree`/`-/blob` and Bitbucket `src/branch` markers. Configurable extra `gitlab_hosts`/`azure_hosts` for self-hosted instances.
- Hardened path handling: `validateRepoSubdir` iteratively URL-decodes up to 3 rounds and rejects NUL/control chars, backslashes, absolute paths, and `.`/`..` traversal segments at every decode layer — closes encoded-traversal bypasses (`%2e%2e`, double-encoding). `validateSourceName`/`validateCloneURL` similarly sanitize derived names and URLs before they touch the filesystem or shell.
- **Tracked repos** (`--track`): clones into `<source>/_<repo-name>/` (leading-underscore convention marks a directory as a live git clone rather than a flat skill), preserving `.git` for future `update` pulls. Clone strategy prefers `--filter=blob:none --depth 1 --single-branch` (partial/shallow clone) and even sparse-checkout for subdir installs, falling back to full clone when the remote lacks these capabilities (capability-string sniffing in `shouldFallbackTrackedClone`). Auto-appends the tracked dir to `.gitignore` so its contents aren't accidentally committed into the user's own skills repo.
- Every install/update runs a **security audit** before or after the operation completes (see below), and the tracked-repo directory is git-managed so `update` = `git pull` + re-audit + re-discovery.

**Metadata/provenance** — `internal/install/meta.go`: each skill gets `.skillshare-meta.json` (source URL, type, install timestamp, repo URL, subdir, git commit/tree hash, per-file SHA-256 hashes). This is what powers conflict detection, `skillshare update --all`, and drift detection.

**Uninstall**: not read in detail, but `internal/trash/trash.go` exists — uninstalls appear to go through a trash/recoverable-delete rather than a hard `rm`.

### 4. Clever ideas worth adopting

- **Data-driven target registry** (`targets.yaml`, 66 entries) cleanly separates "which tools exist and where do they read from" from sync logic — trivially extensible, and `also_scans` captures the reality that some tools scan multiple directories.
- **`.skillignore`/`.agentignore`** — full gitignore-syntax filtering per source repo, independent of per-target include/exclude, giving repo authors control over what ships regardless of consumer config.
- **Manifest + checksum + mtime fast-path** cleanly separates "skillshare-managed" from "user-local" content in copy mode, and the two-tier change detection (cheap mtime check before expensive checksum) is a good efficiency pattern for large skill sets.
- **Provenance-based conflict resolution** (compare `repo_url` in metadata) turns "already exists" from a blind error into "same repo → skip gracefully" vs "different repo → named conflict," which is much friendlier than either always-erroring or always-overwriting.
- **Built-in security audit gating install/update** (`internal/audit/*`) — static + dataflow + tier + integrity + structure + cross-skill + metadata analyzers scanning for prompt injection, exfiltration, hardcoded credentials, obfuscation (hidden Unicode, data URIs), and destructive/shell-execution patterns, with a configurable block threshold (CRITICAL/HIGH/MEDIUM/LOW/INFO) and SARIF export (`internal/audit/sarif.go`). This is the standout feature for a manager that pulls community-authored skills — worth adopting even a subset of it.
- **Extension transforms** (`internal/sync/extension.go`) — pluggable subprocess converters (single executable, or dir + `extension.yaml` with `run`/`output_ext`) that pipe a source file through stdin/stdout during sync, letting the tool emit native formats per target (e.g., Markdown → Gemini TOML commands, Codex TOML agents) rather than forcing every target to consume the same file format. Sandboxed with a 30s timeout.
- **git_root scope selection** (`ScopeDir`/`EffectiveGitRoot`, `config.go:342-390`) — lets `commit`/`push`/`pull` operate on skills, agents, extras, or the whole base dir as one repo, and detects when the actual `.git` lives at a different scope than configured (`GitRootMismatch`) rather than silently failing.
- **Robust URL-decode traversal guard** in `validateRepoSubdir` (multi-round decode-then-validate) is a genuinely careful defense against encoded path traversal — good reference implementation if the new app accepts subdir paths from install URLs.

### Weaknesses / things to avoid

- **Legacy-field sprawl**: config schema has accumulated several generations of the same setting (flat `Path/Mode/Include/Exclude` → `Skills:`/`Agents:` sub-keys; flat `source:` → `sources.skills:`), each requiring migration functions, dual read paths, and "mixed format" merge logic that's now permanently part of the codebase (`config.go:148-187`, `project.go:37-95`). A clean rebuild should pick one schema version and version-bump instead of layering compat shims indefinitely — this itself is presumably close to vstack's own [[no-legacy-support-policy]] stance.
- **JSON manifest sits inside the synced directory** (`.skillshare-manifest.json` in each copy-mode target) — works, but couples state to a location a user could accidentally delete/gitignore/edit; consider a state dir outside the managed tree.
- **Filesystem-walk-heavy discovery**: skills are discovered by walking the whole source tree on nearly every command (`DiscoverSourceSkills*` variants), with several near-duplicate functions differing only in which extra data they collect (frontmatter, ignore stats, context/token counts, tracked-repo paths) — a sign the discovery API grew organically; a single discovery pass with composable post-processing would be tidier for a fresh implementation.
- **No registry/marketplace backend** — despite "hub" naming (`internal/hub/`, `HubConfig` with saved sources), it appears to be just a curated list of known install URLs (`skillshare hub`), not an actual package registry with versioning/publishing. If org-wide discovery matters for the new app, this is a gap, not a pattern to copy.

### Key files for follow-up

- Config/scoping: `internal/config/config.go`, `internal/config/project.go`, `internal/config/targets.go`, `internal/config/targets.yaml`, `internal/config/registry.go`
- Sync/link engine: `internal/sync/sync.go`, `internal/sync/copy.go`, `internal/sync/manifest.go`, `internal/sync/target_naming.go`, `internal/sync/filter.go`, `internal/sync/extension.go`
- Install/provenance: `internal/install/install.go`, `internal/install/source.go`, `internal/install/install_tracked.go`, `internal/install/meta.go`
- Ignore rules: `internal/skillignore/skillignore.go`
- Security audit: `internal/audit/audit.go` (+ sibling `analyzer_*.go` files)
- Symlink/junction abstraction: `internal/utils/link.go`, `internal/utils/link_unix.go` (Windows variant `link_windows.go` not inspected)