//! `guard import-v1` — the one explicit conversion from v1's env-style
//! guard settings to the `[guards]` tables (settled decision 7). Baseline
//! TSVs are shape-identical and read in place, untouched. Excludes files
//! are not translated — equivalence between glob dialects cannot be proven
//! for paths that do not exist yet — they are marked as imported and keep
//! v1's legacy-glob semantics.

use std::collections::BTreeMap;

use crate::error::Result;

use super::{GuardCtx, guard_err, patterns};

const CHECK: &str = "import-v1";

/// The v1 keys with a v2 home, and where each lands.
const CONVERSIONS: [(&str, &str, &str); 9] = [
    ("SIZE_RATCHET_THRESHOLD", "size-ratchet", "threshold"),
    ("SIZE_RATCHET_BASELINE", "size-ratchet", "baseline"),
    ("SIZE_RATCHET_EXCLUDES", "size-ratchet", "excludes"),
    ("GROWTH_GUARDS_TODO_EXCLUDES", "todo-ban", "excludes"),
    (
        "GROWTH_GUARDS_BYTE_CEILING_KB",
        "byte-ceiling",
        "ceiling-kb",
    ),
    ("GROWTH_GUARDS_BYTE_EXCLUDES", "byte-ceiling", "excludes"),
    (
        "GROWTH_GUARDS_SUPPRESSION_BASELINE",
        "suppression-ban",
        "baseline",
    ),
    (
        "GROWTH_GUARDS_SUPPRESSION_EXCLUDES",
        "suppression-ban",
        "excludes",
    ),
    ("GROWTH_GUARDS_COMMIT_TYPES", "commit-msg", "types"),
];

#[derive(Debug)]
pub struct ImportReport {
    pub lines: Vec<String>,
    pub changed: bool,
}

/// Every string assignment in the file, matched file-wide the way v1's
/// reader matched — table structure ignored on purpose.
fn flat_strings(table: &toml::Table) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut walk = vec![table];
    while let Some(table) = walk.pop() {
        for (key, value) in table {
            match value {
                toml::Value::String(text) => {
                    out.entry(key.clone()).or_insert_with(|| text.clone());
                }
                toml::Value::Table(nested) => walk.push(nested),
                _ => {}
            }
        }
    }
    out
}

/// v1 `SIZE_RATCHET_CLASSES` — `pattern=threshold;…` — as the ordered
/// class array, dialect preserved as legacy-glob.
fn classes_toml(raw: &str) -> Result<String> {
    let rows: Vec<String> = super::settings::parse_class_entries(CHECK, raw)?
        .into_iter()
        .map(|(pattern, threshold)| {
            format!(
                "  {{ pattern = {}, threshold = {threshold} }},",
                toml_string(&pattern)
            )
        })
        .collect();
    Ok(format!("classes = [\n{}\n]\n", rows.join("\n")))
}

fn toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Convert the settings file in the working tree — this is the explicit,
/// user-invoked conversion, and the working tree is where the user will
/// review and commit it. Legacy keys stay where they are so v1 tooling
/// keeps working through the transition; the `[guards]` tables are
/// appended, byte-faithfully, at the end of the file.
pub fn run(ctx: &GuardCtx) -> Result<ImportReport> {
    let mut report = ImportReport {
        lines: Vec::new(),
        changed: false,
    };
    let path = ctx.root.join("vstack.settings.toml");
    let Some(text) = crate::fs::read_if_exists(&path)? else {
        report
            .lines
            .push("no vstack.settings.toml here — nothing to convert".into());
        return Ok(report);
    };
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| {
        guard_err(CHECK, format!("vstack.settings.toml: invalid TOML: {e}"))
    })?;
    if table.contains_key("guards") {
        report.lines.push(
            "vstack.settings.toml already carries [guards] tables — nothing converted".into(),
        );
        return Ok(report);
    }
    let flat = flat_strings(&table);
    let (sections, mut excludes_files) = convert_sections(&flat, &mut report)?;
    if sections.is_empty() {
        report
            .lines
            .push("no legacy guard settings found — nothing converted".into());
        return Ok(report);
    }

    // Default excludes files count as referenced even when no key names
    // them: v1 read the same defaults.
    for default in [
        "tools/size-ratchet-excludes",
        "tools/todo-ban-excludes",
        "tools/byte-ceiling-excludes",
        "tools/suppression-ban-excludes",
    ] {
        if !excludes_files.iter().any(|file| file == default) {
            excludes_files.push(default.to_owned());
        }
    }
    for file in &excludes_files {
        mark_excludes_imported(ctx, file, &mut report)?;
    }

    let mut updated = text.clone();
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    for (check, entries) in &sections {
        updated.push_str(&format!("\n[guards.{check}]\n"));
        for entry in entries {
            updated.push_str(entry);
        }
    }
    crate::fs::atomic_write(&path, &updated)?;
    report.changed = true;
    report.lines.push(
        "vstack.settings.toml: [guards] tables appended — review and commit the change".into(),
    );
    Ok(report)
}

/// The key-by-key conversion: which `[guards]` entries the legacy values
/// become, and which excludes files they reference.
#[allow(clippy::type_complexity)]
fn convert_sections(
    flat: &BTreeMap<String, String>,
    report: &mut ImportReport,
) -> Result<(BTreeMap<&'static str, Vec<String>>, Vec<String>)> {
    let mut sections: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let mut excludes_files: Vec<String> = Vec::new();
    for (legacy, check, key) in CONVERSIONS {
        let Some(value) = flat.get(legacy) else {
            continue;
        };
        let rendered = match (check, key) {
            ("size-ratchet", "threshold") | ("byte-ceiling", "ceiling-kb") => {
                let number: u64 = value.trim().parse().map_err(|_| {
                    guard_err(
                        CHECK,
                        format!("{legacy} is not a positive integer: '{value}'"),
                    )
                })?;
                format!("{key} = {number}\n")
            }
            ("commit-msg", "types") => format!(
                "{key} = [{}]\n",
                value
                    .split_whitespace()
                    .map(toml_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => format!("{key} = {}\n", toml_string(value)),
        };
        if key == "excludes" {
            excludes_files.push(super::settings::config_path(CHECK, value)?);
        }
        sections.entry(check).or_default().push(rendered);
        report
            .lines
            .push(format!("converted {legacy} -> [guards.{check}] {key}"));
    }
    if let Some(raw) = flat.get("SIZE_RATCHET_CLASSES") {
        sections
            .entry("size-ratchet")
            .or_default()
            .push(classes_toml(raw)?);
        sections
            .entry("size-ratchet")
            .or_default()
            .push("classes-dialect = \"legacy-glob\"\n".to_owned());
        report
            .lines
            .push("converted SIZE_RATCHET_CLASSES -> [guards.size-ratchet] classes (legacy-glob dialect kept)".into());
    }
    if flat.contains_key("GROWTH_GUARDS_PRE_COMMIT_LOCAL") {
        report.lines.push(
            "GROWTH_GUARDS_PRE_COMMIT_LOCAL was not converted: a repository file must never name the executable the chain runs — set VSTACK_GUARD_PRE_COMMIT_LOCAL in your machine's environment instead"
                .into(),
        );
    }
    Ok((sections, excludes_files))
}

/// Prepend the legacy-dialect marker to an existing excludes file that
/// lacks it: the patterns keep the semantics they were written in.
fn mark_excludes_imported(ctx: &GuardCtx, file: &str, report: &mut ImportReport) -> Result<()> {
    let path = ctx.root.join(file);
    let Some(text) = crate::fs::read_if_exists(&path)? else {
        return Ok(());
    };
    if text.lines().next().map(str::trim) == Some(patterns::LEGACY_DIALECT_MARKER) {
        return Ok(());
    }
    let updated = format!("{}\n{text}", patterns::LEGACY_DIALECT_MARKER);
    crate::fs::atomic_write(&path, &updated)?;
    report.changed = true;
    report.lines.push(format!(
        "{file}: marked as imported — its patterns keep v1's legacy-glob semantics"
    ));
    Ok(())
}
