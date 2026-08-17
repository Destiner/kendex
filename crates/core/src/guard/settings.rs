//! The project settings resolver the guards read — built once, in core.
//!
//! Precedence per key: the process environment
//! (`VSTACK_GUARDS_<CHECK>_<KEY>`), then `.vstack/settings.toml`, then the
//! committed `vstack.settings.toml`, then the built-in default. Everything
//! that decides a verdict is read from the index (settled decision 5): a
//! tracked settings file resolves from the staged copy, one staged for
//! deletion governs as absent, and only a never-tracked file is read from
//! disk — the environment is the one machine-local override.

use crate::error::{CoreError, Result};

use super::ctx::GuardCtx;

/// The two settings files, in precedence order.
const SETTINGS_FILES: [&str; 2] = [".vstack/settings.toml", "vstack.settings.toml"];

fn err(check: &str, message: impl Into<String>) -> CoreError {
    CoreError::Guard {
        check: check.to_owned(),
        message: message.into(),
    }
}

/// A policy file's content as the commit carries it. `None` = the commit
/// has no such file. Tracked → the index copy (a read failure is loud);
/// in HEAD but not the index → staged for deletion, absent; never
/// tracked → the worktree copy, which is all there is.
pub fn policy_content(ctx: &GuardCtx, check: &str, path: &str) -> Result<Option<Vec<u8>>> {
    let tracked = ctx
        .git(&["ls-files", "--error-unmatch", "--", path])
        .run()?
        .status
        .success();
    if tracked {
        return match ctx.index_content(check, path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => Err(err(
                check,
                format!("could not read the staged copy of {path}"),
            )),
        };
    }
    let in_head = ctx
        .git(&["cat-file", "-e", &format!("HEAD:{path}")])
        .run()?
        .status
        .success();
    if in_head {
        return Ok(None);
    }
    let on_disk = ctx.root.join(path);
    match on_disk.is_file() {
        true => Ok(Some(
            std::fs::read(&on_disk).map_err(|e| CoreError::io(&on_disk, e))?,
        )),
        false => Ok(None),
    }
}

/// The `[guards]` tables of both settings files, resolved per key.
#[derive(Debug)]
pub struct Policy {
    /// `(file, guards-table)` in precedence order; files without a
    /// `[guards]` table carry an empty one.
    tables: Vec<(String, toml::Table)>,
}

impl Policy {
    /// Load both settings files through the index-aware read. A parse
    /// failure is exit 2 — a guard must never fall back past a file that
    /// is present but unreadable. Legacy `SIZE_RATCHET_*` /
    /// `GROWTH_GUARDS_*` settings with no `[guards]` tables anywhere are a
    /// refusal naming the conversion command (settled decision 7).
    pub fn load(ctx: &GuardCtx, check: &str) -> Result<Policy> {
        // Every check loads policy first, so this is the one place the
        // unjudgeable-index refusal is guaranteed to run per lane.
        ctx.assert_judgeable(check)?;
        let mut tables = Vec::new();
        let mut legacy_in: Vec<String> = Vec::new();
        for file in SETTINGS_FILES {
            let Some(bytes) = policy_content(ctx, check, file)? else {
                continue;
            };
            let text = String::from_utf8_lossy(&bytes);
            let parsed: toml::Table = text
                .parse()
                .map_err(|e: toml::de::Error| err(check, format!("{file}: invalid TOML: {e}")))?;
            if text.contains("SIZE_RATCHET_") || text.contains("GROWTH_GUARDS_") {
                legacy_in.push(file.to_owned());
            }
            let guards = parsed
                .get("guards")
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            tables.push((file.to_owned(), guards));
        }
        // The dotenv layers v1 read are legacy carriers too.
        for file in [".env.local", ".env"] {
            if let Some(bytes) = policy_content(ctx, check, file)? {
                let text = String::from_utf8_lossy(&bytes);
                if text.contains("SIZE_RATCHET_") || text.contains("GROWTH_GUARDS_") {
                    legacy_in.push(file.to_owned());
                }
            }
        }
        let converted = tables.iter().any(|(_, guards)| !guards.is_empty());
        if !legacy_in.is_empty() && !converted {
            return Err(err(
                check,
                format!(
                    "legacy v1 guard settings found in {} with no [guards] tables — convert them once with the guard import-v1 command",
                    legacy_in.join(", ")
                ),
            ));
        }
        Ok(Policy { tables })
    }

    fn env_override(check: &str, key: &str) -> Option<String> {
        let name = format!(
            "VSTACK_GUARDS_{}_{}",
            check.replace('-', "_").to_ascii_uppercase(),
            key.replace('-', "_").to_ascii_uppercase()
        );
        std::env::var(&name).ok()
    }

    fn file_value(&self, check: &str, key: &str) -> Option<&toml::Value> {
        self.tables.iter().find_map(|(_, guards)| {
            guards
                .get(check)
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get(key))
        })
    }

    pub fn string(&self, check: &str, key: &str, default: &str) -> Result<String> {
        if let Some(value) = Self::env_override(check, key) {
            return Ok(value);
        }
        match self.file_value(check, key) {
            None => Ok(default.to_owned()),
            Some(toml::Value::String(value)) => Ok(value.clone()),
            Some(other) => Err(err(
                check,
                format!("[guards.{check}] {key} must be a string, got {other}"),
            )),
        }
    }

    pub fn positive_int(&self, check: &str, key: &str, default: u64) -> Result<u64> {
        let parsed: Option<u64> = match Self::env_override(check, key) {
            Some(value) => Some(value.parse().map_err(|_| {
                err(
                    check,
                    format!("{key} must be a positive integer, got '{value}'"),
                )
            })?),
            None => match self.file_value(check, key) {
                None => None,
                Some(toml::Value::Integer(value)) if *value > 0 => Some(*value as u64),
                Some(other) => {
                    return Err(err(
                        check,
                        format!("[guards.{check}] {key} must be a positive integer, got {other}"),
                    ));
                }
            },
        };
        match parsed {
            Some(0) => Err(err(
                check,
                format!("{key} must be a positive integer, got 0"),
            )),
            Some(value) => Ok(value),
            None => Ok(default),
        }
    }

    /// A list: TOML array of strings in a file, whitespace-separated in the
    /// environment override.
    pub fn string_list(&self, check: &str, key: &str, default: &[&str]) -> Result<Vec<String>> {
        if let Some(value) = Self::env_override(check, key) {
            return Ok(value.split_whitespace().map(str::to_owned).collect());
        }
        match self.file_value(check, key) {
            None => Ok(default.iter().map(|s| (*s).to_owned()).collect()),
            Some(toml::Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_str().map(str::to_owned).ok_or_else(|| {
                        err(
                            check,
                            format!("[guards.{check}] {key} must be an array of strings"),
                        )
                    })
                })
                .collect(),
            Some(other) => Err(err(
                check,
                format!("[guards.{check}] {key} must be an array of strings, got {other}"),
            )),
        }
    }

    pub fn enabled(&self, check: &str) -> Result<bool> {
        if let Some(value) = Self::env_override(check, "enabled") {
            return Ok(value != "false" && value != "0" && value != "off");
        }
        match self.file_value(check, "enabled") {
            None => Ok(true),
            Some(toml::Value::Boolean(value)) => Ok(*value),
            Some(other) => Err(err(
                check,
                format!("[guards.{check}] enabled must be a boolean, got {other}"),
            )),
        }
    }

    /// Ordered path classes — an array of `{ pattern, threshold }` rules,
    /// first match wins. A plain TOML table is unordered, and ordering is
    /// load-bearing for thresholds, so only the array form is accepted. The
    /// environment override keeps v1's `pattern=threshold;…` spelling.
    pub fn classes(&self, check: &str) -> Result<Vec<(String, u64)>> {
        if let Some(value) = Self::env_override(check, "classes") {
            return value
                .split(';')
                .filter(|entry| !entry.trim().is_empty())
                .map(|entry| {
                    let (pattern, threshold) = entry.split_once('=').ok_or_else(|| {
                        err(
                            check,
                            format!("class entry '{entry}' is not pattern=threshold"),
                        )
                    })?;
                    let threshold = threshold.trim().parse().map_err(|_| {
                        err(
                            check,
                            format!("class threshold in '{entry}' is not a positive integer"),
                        )
                    })?;
                    Ok((pattern.trim().to_owned(), threshold))
                })
                .collect();
        }
        match self.file_value(check, "classes") {
            None => Ok(Vec::new()),
            Some(toml::Value::Array(items)) => items
                .iter()
                .map(|item| {
                    let table = item.as_table().ok_or_else(|| {
                        err(
                            check,
                            format!("[guards.{check}] classes entries must be tables"),
                        )
                    })?;
                    let pattern = table
                        .get("pattern")
                        .and_then(toml::Value::as_str)
                        .ok_or_else(|| {
                            err(
                                check,
                                format!("[guards.{check}] class entry needs a pattern"),
                            )
                        })?;
                    let threshold = table
                        .get("threshold")
                        .and_then(toml::Value::as_integer)
                        .filter(|value| *value > 0)
                        .ok_or_else(|| {
                            err(
                                check,
                                format!("[guards.{check}] class entry needs a positive threshold"),
                            )
                        })?;
                    Ok((pattern.to_owned(), threshold as u64))
                })
                .collect(),
            Some(other) => Err(err(
                check,
                format!("[guards.{check}] classes must be an array of tables, got {other}"),
            )),
        }
    }
}

/// A configured repo-relative path, validated: absolute paths, escapes,
/// and names shaped like options are configuration errors.
pub fn config_path(check: &str, raw: &str) -> Result<String> {
    if raw.starts_with('/') {
        return Err(err(
            check,
            format!("path must be repo-root-relative, got absolute: {raw}"),
        ));
    }
    let mut out: Vec<&str> = Vec::new();
    for segment in raw.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if out.pop().is_none() {
                    return Err(err(check, format!("path escapes the repository: {raw}")));
                }
            }
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return Err(err(check, format!("path normalizes empty: {raw}")));
    }
    let joined = out.join("/");
    if joined.starts_with('-') {
        return Err(err(
            check,
            format!("path must not begin with '-': {joined}"),
        ));
    }
    Ok(joined)
}
