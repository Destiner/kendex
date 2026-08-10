# vstack v1 CLI compatibility contract
Source of truth: `~/dev/vstack` @ `169eff98`. All paths below are repo-relative; line numbers cite that commit.

## Binary-level (cli/src/main.rs)

| Item | Value | Cite |
|---|---|---|
| name | `vstack` | main.rs:42 |
| version string | `"{CARGO_PKG_VERSION} ({VSTACK_GIT_HASH})"` via `const_format()` | main.rs:32,34-38,43 |
| about | `"Skills, agents, hooks. Cross-harness."` | main.rs:44 |
| subcommand | `Option<Commands>` — optional; no subcommand falls through to `add` | main.rs:47-48,440 |
| true global flags | none besides clap's auto `-h/--help`, `-V/--version`. The top-level flags below exist only to make the bare form work and are forwarded to `add` | main.rs:50-104 |
| error → exit code | `main() -> anyhow::Result<()>`; any `Err`/`bail!` prints `Error: ...` to stderr and exits **1** (Rust `Termination`). Clap parse errors exit **2** (clap default). | main.rs:318 |

## Bare form: `vstack <source> [flags]` → `add`

The top-level `Cli` struct carries an optional positional `source` (main.rs:52) plus the identical 11 flags as `Add` (main.rs:54-104, comment at :50 "Top-level flags that map to `add` when no subcommand given"). Dispatch: `None => commands::add::run(cli.source, cli.global, cli.harness, cli.agent, cli.skill, cli.hook, cli.pi_extension, cli.copy, cli.yes, cli.all, cli.clobber, cli.no_auto_skills)` — main.rs:440-453. So `vstack owner/repo -g -y` ≡ `vstack add owner/repo -g -y`, flag-for-flag.

## `add` (main.rs:110-141; run: cli/src/commands/add.rs:1770)

| Flag | Short | Type | Default | Meaning |
|---|---|---|---|---|
| `source` (positional) | — | `Option<String>` | none | GitHub `owner/repo` or local path |
| `--global` | `-g` | bool | false | install to user-level dir |
| `--harness` | — | `Option<Vec<String>>`, comma-delimited | none | targets: claude,cursor,opencode,codex,pi |
| `--agent` | `-a` | `Option<Vec<String>>`, comma-delimited | none | install specific agents |
| `--skill` | `-s` | `Option<Vec<String>>`, comma-delimited | none | install specific skills |
| `--hook` | — | `Option<Vec<String>>`, comma-delimited | none | install specific hooks |
| `--pi-extension` | — | `Option<Vec<String>>`, comma-delimited; **visible alias `--pi-package`** | none | install specific Pi extensions |
| `--copy` | — | bool | false | copy instead of symlink |
| `--yes` | `-y` | bool | false | skip confirmation prompts |
| `--all` | — | bool | false | all items to all harnesses |
| `--clobber` | — | bool | false | allow `--global --all` over a non-empty global lock |
| `--no-auto-skills` | — | bool | false | skip auto-install of skills referenced by selected agents (default walks `agent-skills` + `role-skills` + transitive deps) |

Exit: guards bail (exit 1): `--global` non-interactive without `--all`/item filter (add.rs:1795-1811); `--global --all` over populated global lock without `--clobber` (add.rs:1821+, bail ~:1863). Success 0.

## `apply` (main.rs:144-165; run: cli/src/commands/apply.rs:185)

| Flag | Short | Type | Default | Meaning |
|---|---|---|---|---|
| `extra_name` (positional) | — | `String`, **required** | — | extra name from current source |
| `--theme` | — | `Option<String>` | extra's default theme | theme id |
| `--target` | — | `Option<String>` (single string, comma-separated inside) | all detected declared targets | target subset |
| `--global` | — | bool (no `-g`!) | false | user/global scope |
| `--dry-run` | — | bool | false | print planned changes, write nothing |
| `--no-ghostty-shaders` | — | bool | false | skip Ghostty shader lines/files |
| `--yes` | `-y` | bool | false | skip confirm prompt |

Exit: dry-run returns 0 after printing plan (apply.rs:214-216); declined prompt → `bail!("apply cancelled")` exit 1 (apply.rs:218-220); unknown extra/target errors exit 1.

## `remove` (main.rs:167-176; run: cli/src/commands/remove.rs:7)

| Flag | Short | Type | Default | Meaning |
|---|---|---|---|---|
| `names` (positional) | — | `Vec<String>` | empty | items to remove |
| `--global` | `-g` | bool | false | shortcut for `--scope global` |
| `--scope` | — | `Option<String>` | **project** | project \| global \| all |

Exit: empty `names` → prints usage, exit **0** (remove.rs:8-11). Per-scope removal failures → bail exit 1 (remove.rs:147-153). Names not found in any lock → "Nothing removed" message but exit **0** (remove.rs:160-165).

## `list` (alias `ls`) (main.rs:178-191; run: cli/src/commands/list.rs:6)

| Flag | Short | Type | Default | Meaning |
|---|---|---|---|---|
| `--global` | `-g` | bool | false | shortcut for `--scope global` |
| `--scope` | — | `Option<String>` | **all** | project \| global \| all |
| `--harness` | — | `Option<String>` (single value, no delimiter) | none | filter by harness id |

Alias declared `#[command(alias = "ls")]` main.rs:180. Exit: always 0 unless I/O error.

## `check` (main.rs:193-202; run: cli/src/commands/check.rs:123)

| Flag | Short | Type | Default | Meaning |
|---|---|---|---|---|
| `--global` | `-g` | bool | false | shortcut for `--scope global` |
| `--scope` | — | `Option<String>` | **all** | project \| global \| all |

Behavior: prints CLI version + remote-update hint (check.rs:125-138); per scope reports outdated (`!`), orphaned on-disk-not-in-lock, phantom in-lock-not-on-disk (check.rs:186-232); scans agents for frontmatter skills not installed (check.rs:239-294).
Exit: **outdated/orphaned/phantom items alone exit 0**. Non-zero (bail, exit 1) **only** when ≥1 agent references an uninstalled skill: `bail!("{missing_skill_refs} skill reference(s) missing from install...")` check.rs:297-301.

## `update` (main.rs:204-211; run: cli/src/commands/update.rs:9)

| Flag | Short | Type | Default | Meaning |
|---|---|---|---|---|
| `--force` | `-f` | bool | false | reinstall even if version matches |

Self-updates the binary via `cargo install --git https://github.com/vanillagreencom/vstack.git vstack --force` (update.rs:35-44). Exit: already-latest without `--force` → 0 (update.rs:18-21); `cargo install` failure → bail exit 1 (update.rs:49).

## `refresh` (main.rs:213-228; run: cli/src/commands/refresh.rs:1066)

| Flag | Short | Type | Default | One-line meaning |
|---|---|---|---|---|
| `--global` | `-g` | bool | false | shortcut for `--scope global` |
| `--scope` | — | `Option<String>` | **all** | which scope(s) to refresh: project \| global \| all |
| `--verbose` | `-v` | bool | false | print per-item `old-hash → new-hash` with changed/unchanged/failed/missing status (refresh.rs:1329-1389) instead of the compact "! kind updated: names" summary (1390-1408) |

There is **no `--dry-run` on refresh** — the full flag set is exactly the three above (main.rs:218-228).
Exit: nothing installed → message + 0 (refresh.rs:1086-1089); any per-item/harness install failure → bail `"failed to refresh N item/harness install(s)"` exit 1 (1443-1456); any locked item missing from its source → bail exit 1 (1457-1463); else 0.

## `verify` (main.rs:230-244; run: cli/src/commands/verify.rs:41)

| Flag | Short | Type | Default | Meaning |
|---|---|---|---|---|
| `names` (positional) | — | `Vec<String>` | empty = all installed items | filter by name |
| `--global` | `-g` | bool | false | shortcut for `--scope global` |
| `--scope` | — | `Option<String>` | **all** | project \| global \| all |

Semantics: per lock entry checks (a) source hash still matches lock (`source_ok`, verify.rs:119-126) and (b) per-kind install check — Pi packages byte-compare install dir vs source dir (verify.rs:146-158); skills/agents/hooks have their own checks; Extras skipped (`·`). A row fails when `!source_ok || install_ok == Some(false) || note.is_some()` (verify.rs:75) — so an unresolvable source path counts as failure.
Exit: **`std::process::exit(1)` when any row failed** (verify.rs:108-109) — this is the drift signal; nothing installed → 0 (verify.rs:97-99); all OK → 0. Output goes to **stderr** (`eprintln!`), summary line `"N checked, N OK, N failed"` (verify.rs:102-107).

## `update-pi` (clap name from `UpdatePi`; main.rs:246-256; run: cli/src/commands/update_pi.rs:538)

| Flag | Short | Type | Default | One-line meaning |
|---|---|---|---|---|
| `--check` | `-c` | bool | false | print plan only (stale/outdated packages), modify nothing, and exit 0 even when updates exist (update_pi.rs:547-558) |
| `--scope` | — | `Option<String>` | **all** | restrict to one scope: all \| global \| project (parsed via `ScopeFilter`, update_pi.rs:51-54) |

Exit: `--check` always 0; without it, any failed package update → bail `"update failed for: ..."` exit 1 (update_pi.rs:502-504); vstack-source package not currently installed at that scope aborts that item (`update_pi.rs:510`) and counts as failure.

## `init` (main.rs:258-265; run: cli/src/commands/init.rs:29)

| Flag | Short | Type | Default | Meaning |
|---|---|---|---|---|
| `name` (positional) | — | `Option<String>` | none | new item name |
| `--kind` | — | `Option<String>` | none | agent \| skill \| hook (aliases: agents/a, skills/s, hooks/h — init.rs:20-24) |

Exit: no name → usage, exit **0** (init.rs:30-40); name without `--kind` → bail exit 1 (init.rs:41-48); name containing `/` or leading `-` → bail exit 1 (init.rs:51-53).

## `report` (main.rs:267-315; run: cli/src/commands/report.rs:227)

| Flag | Short | Type | Default | One-line meaning |
|---|---|---|---|---|
| `--skill` | — | `Option<String>` | none | report about an installed skill by name |
| `--agent` | — | `Option<String>` | none | report about an installed agent by name |
| `--hook` | — | `Option<String>` | none | report about an installed hook by name |
| `--asset` | — | `Option<String>` | none | any installed asset by name, kind auto-detected |
| `--title` | — | `String` **required** | — | issue title |
| `--body` | — | `Option<String>` | none | issue body text (mutually exclusive with `--body-file`) |
| `--body-file` | — | `Option<PathBuf>` | none | read body from file (mutually exclusive with `--body`) |
| `--global` | `-g` | bool | false | shortcut for `--scope global` |
| `--scope` | — | `Option<String>` | **project** | ownership-resolution scope: project \| global — **`all` rejected** (report.rs:558-565) |
| `--upstream` | — | `Option<String>` | `vanillagreencom/vstack` (report.rs:242-244) | upstream repo for vstack-owned issues |
| `--area` | — | `Option<String>` | derived from selector | routing label: cli \| skills \| harness \| review-gate \| docs \| tech-debt; validated up-front (report.rs:245-247) |
| `--dry-run` | — | bool | false | print ownership decision, target repo, and exact `gh` command; file nothing; exit 0 (report.rs:278-281) |

Validation → exit 1: more than one of `--skill/--agent/--hook/--asset` (report.rs:529-535); both or neither of `--body`/`--body-file` (report.rs:540-551); `--scope all` (report.rs:562-564); bad `--area`. Zero selectors is allowed with a warning and defaults ownership to local (report.rs:254-259). Filing via `gh` fails → saves body to disk, prints guidance, bail exit 1 (report.rs:292-302). Success prints `Issue filed[: url]` to **stdout** (report.rs:284-290) and exits 0.

## Notes

- **Scope resolution (shared)**: `--scope` beats `--global` when both given; `--global` alone = global; otherwise per-command default (`ScopeFilter::resolve`, cli/src/scope.rs:22-30). Parser accepts aliases: `project|p|local`, `global|g|user`, `all|both|*` (scope.rs:51-59); unknown value → error exit 1. Defaults: `Project` for remove and report; `All` for list, check, refresh, verify, update-pi.
- **Scope default table**: remove=project, report=project(+rejects all), list/check/refresh/verify/update-pi=all.
- **stdout vs stderr**: nearly all human output (list/check/refresh/verify tables, prompts) is `eprintln!` (stderr); stdout is used for apply's rendered plan (apply.rs:212), report success line, and update-pi progress lines.
- **`--pi-package`** is a visible alias of `--pi-extension` on both the bare form (main.rs:75) and `add` (main.rs:127).
- **`ls`** is the only subcommand alias in the CLI (main.rs:180).
- Exit-code drift contract, condensed: `verify` = hard `exit(1)` on any drift row; `refresh` = 1 on any failure or source-missing item (not on mere changes); `check` = 1 only for agent→missing-skill references; `update-pi --check` = always 0; `report --dry-run`/`apply --dry-run` = 0.
- `pi_extension`/`no_auto_skills`/`dry_run`/`no_ghostty_shaders`/`body_file` render as kebab-case long flags per clap default; `UpdatePi` renders as `update-pi`.