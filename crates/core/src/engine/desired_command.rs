use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::error::Result;
use crate::frontmatter::Value;
use crate::lock::EmittedArtifact;
use crate::model::{HarnessId, ItemKind};
use crate::render::agent::GENERATED_BANNER;
use crate::render::yaml_scalar;

use super::ItemWarning;
use super::desired::{
    Artifact, Desired, DesiredState, ItemCtx, native_dir, refusal_reason, target_harnesses,
};
use super::desired_kinds::declared;
use super::targets::disabled_name;

pub(super) fn desired_command(ctx: &ItemCtx, state: &mut DesiredState) -> Result<()> {
    let bytes = ctx.sealed.read(ctx.item_path)?;
    for harness in ctx.harnesses.clone() {
        let item = match crate::harness::capabilities(harness, ItemKind::Command).installs_as {
            None => native_file(ctx, harness, &bytes)?,
            Some(ItemKind::Skill) => as_skill(ctx, state, harness, &bytes)?,
            Some(kind) => {
                state.notes.push(format!(
                    "command {}: {} stores commands as {}s, which vstack cannot write",
                    ctx.name,
                    harness.display_name(),
                    kind.name()
                ));
                None
            }
        };
        state.items.extend(item);
    }
    Ok(())
}

/// The harness reads commands from a directory of its own: one file, named
/// for the command, parked under `.disabled` while it is turned off.
fn native_file(ctx: &ItemCtx, harness: HarnessId, bytes: &[u8]) -> Result<Option<Desired>> {
    let Some(dir) = native_dir(ctx.env, ctx.scope, harness, ItemKind::Command) else {
        return Ok(None);
    };
    let file = dir.join(format!("{}.md", ctx.name));
    let artifact = Artifact::File {
        path: match ctx.decl.enabled {
            true => file,
            false => disabled_name(&file),
        },
        bytes: bytes.to_vec(),
    };
    Ok(Some(declared(ctx, ItemKind::Command, harness, artifact)?))
}

/// Codex has no command directory to write to — it retired prompts in favor
/// of skills — so the command becomes a one-file skill tree on the skill
/// surface, and the lock carries the name and path it took.
fn as_skill(
    ctx: &ItemCtx,
    state: &mut DesiredState,
    harness: HarnessId,
    bytes: &[u8],
) -> Result<Option<Desired>> {
    let Some(dir) = native_dir(ctx.env, ctx.scope, harness, ItemKind::Skill) else {
        return Ok(None);
    };
    let Some(name) = emitted_name(ctx, state, harness) else {
        return Ok(None);
    };
    let body = String::from_utf8_lossy(bytes);
    let tree = dir.join(&name);
    let mut files = vec![(
        PathBuf::from("SKILL.md"),
        skill_text(&name, &body, ctx.name).into_bytes(),
    )];
    // A command is one file the author cannot split themselves, so an
    // oversized one is cut into references/ exactly like a skill — nothing
    // is dropped, and only a body the splitter cannot cut is refused.
    if let Some(cap) = crate::harness::format_caps(harness).skill_body_max_bytes {
        let Some(capped) = split_to_cap(ctx, state, harness, files, cap) else {
            return Ok(None);
        };
        files = capped;
    }
    if !ctx.decl.enabled {
        for (rel, _) in &mut files {
            if rel == std::path::Path::new("SKILL.md") {
                *rel = PathBuf::from("SKILL.md.disabled");
            }
        }
    }
    // Installed as a skill, it answers to the skill loader's rules — under
    // the emitted name, which is the one the user will type.
    let findings = crate::render::validate::validate_skill_tree(harness, &name, &files);
    if let Some(reason) = refusal_reason(&findings) {
        state.refused.push(super::desired::Refused {
            kind: ItemKind::Command,
            name: ctx.name.to_owned(),
            harness,
            reason,
        });
        return Ok(None);
    }
    for finding in findings.iter().filter(|finding| !finding.is_breakage()) {
        state.warnings.push(ItemWarning {
            kind: ItemKind::Command,
            name: ctx.name.to_owned(),
            harness: Some(harness),
            message: finding.message.clone(),
            remediation: Some(finding.remediation.clone()),
        });
    }
    // Pi reads the same project skill directory Codex does, so the generated
    // skill shows up there too. Saying so beats the user finding a command
    // they never declared for Pi in Pi's skill list.
    if native_dir(ctx.env, ctx.scope, HarnessId::Pi, ItemKind::Skill).as_ref() == Some(&dir) {
        state.warnings.push(ItemWarning {
            kind: ItemKind::Command,
            name: ctx.name.to_owned(),
            harness: Some(harness),
            message: format!(
                "installed as skill {name} in a directory Pi also reads, so Pi offers it too"
            ),
            remediation: Some(format!(
                "drop {} from this command's harnesses if Pi must not see it",
                harness.display_name()
            )),
        });
    }
    let artifact = Artifact::Tree {
        canonical: tree.clone(),
        files,
        link: None,
    };
    let mut item = declared(ctx, ItemKind::Command, harness, artifact)?;
    item.emitted = Some(EmittedArtifact {
        kind: ItemKind::Skill,
        name,
        paths: vec![tree],
    });
    Ok(Some(item))
}

/// Cut the generated skill down to the harness's byte cap. `None` means the
/// splitter could not cut it at all — one code block bigger than the cap —
/// and the command is refused for this harness rather than truncated.
fn split_to_cap(
    ctx: &ItemCtx,
    state: &mut DesiredState,
    harness: HarnessId,
    files: Vec<(PathBuf, Vec<u8>)>,
    cap: usize,
) -> Option<Vec<(PathBuf, Vec<u8>)>> {
    let outcome = crate::render::split::enforce_body_cap(files, cap);
    if let Some(reason) = outcome.refusal {
        state.refused.push(super::desired::Refused {
            kind: ItemKind::Command,
            name: ctx.name.to_owned(),
            harness,
            reason: format!("{reason} — break the block up in the command's own file"),
        });
        return None;
    }
    for warning in outcome.warnings {
        state.warnings.push(ItemWarning {
            kind: ItemKind::Command,
            name: ctx.name.to_owned(),
            harness: Some(harness),
            message: warning.message,
            remediation: Some(format!(
                "nothing to fix — {} reads the rest from references/; shorten the command to keep it in one file",
                harness.display_name()
            )),
        });
    }
    Some(outcome.files)
}

/// The name the generated skill takes. A real skill keeps its own name, so
/// a command that clashes is renamed and the user is told which name to
/// type; when both renames are taken too, nothing is written rather than
/// something being overwritten.
fn emitted_name(ctx: &ItemCtx, state: &mut DesiredState, harness: HarnessId) -> Option<String> {
    match emitted_names(ctx, harness).remove(ctx.name).flatten() {
        Some(name) if name == ctx.name => Some(name),
        Some(name) => {
            state.warnings.push(ItemWarning {
                kind: ItemKind::Command,
                name: ctx.name.to_owned(),
                harness: Some(harness),
                message: format!(
                    "{} is already taken on {}, so the command installs as {name}",
                    ctx.name,
                    harness.display_name()
                ),
                remediation: Some(format!(
                    "run it as {name} on {}, or rename one of the two",
                    harness.display_name()
                )),
            });
            Some(name)
        }
        None => {
            state.refused.push(super::desired::Refused {
                kind: ItemKind::Command,
                name: ctx.name.to_owned(),
                harness,
                reason: format!(
                    "{name}, {name}__command and {name}__cmd are all taken on {} — rename one of them",
                    harness.display_name(),
                    name = ctx.name
                ),
            });
            None
        }
    }
}

/// The name every declared command emits on this harness, resolved in one
/// pass so no two commands can pick the same tree. Skills hold their names
/// outright; among commands the first in name order keeps the plain name and
/// later ones take a suffix — a fixed order, so the answer does not depend on
/// which command was rendered first and never changes between audits.
fn emitted_names(ctx: &ItemCtx, harness: HarnessId) -> BTreeMap<String, Option<String>> {
    let mut taken = claimed_skill_names(ctx, harness);
    let mut chosen = BTreeMap::new();
    for (name, decl) in &ctx.manifest.commands {
        if !target_harnesses(decl, ctx.manifest, ItemKind::Command, ctx.scope).contains(&harness) {
            continue;
        }
        let free = free_name(name, &taken);
        if let Some(free) = &free {
            taken.insert(free.clone());
        }
        chosen.insert(name.clone(), free);
    }
    chosen
}

/// The first of `name`, `name__command` and `name__cmd` nothing holds yet.
fn free_name(name: &str, taken: &BTreeSet<String>) -> Option<String> {
    ["", "__command", "__cmd"]
        .into_iter()
        .map(|suffix| format!("{name}{suffix}"))
        .find(|candidate| !taken.contains(candidate))
}

/// Skill names this harness must not have taken from it: the ones declared
/// for it here, plus everything the source offers, since declaring one of
/// those is a single edit away.
fn claimed_skill_names(ctx: &ItemCtx, harness: HarnessId) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = ctx
        .manifest
        .skills
        .iter()
        .filter(|(_, decl)| {
            target_harnesses(decl, ctx.manifest, ItemKind::Skill, ctx.scope).contains(&harness)
        })
        .map(|(name, _)| name.clone())
        .collect();
    names.extend(crate::source::list_items(
        ctx.sealed,
        ctx.config,
        ItemKind::Skill,
    ));
    names
}

/// The generated SKILL.md: the frontmatter the loader needs, the banner
/// every generated file carries, then the command's prose. The command's own
/// frontmatter stays out — carried through, `argument-hint` and
/// `allowed-tools` would read as literal text inside the skill.
fn skill_text(emitted: &str, body: &str, name: &str) -> String {
    let (front, prose) = match crate::frontmatter::split(body) {
        Ok((front, prose)) => (Some(front), prose),
        Err(_) => (None, body),
    };
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n{GENERATED_BANNER}\n\n{}",
        yaml_scalar(emitted),
        yaml_scalar(&description(front, prose, name)),
        prose.trim_start_matches('\n'),
    )
}

/// One line saying what the command does: the `description` its own
/// frontmatter declares, else its first line of prose. The frontmatter is
/// parsed, not scanned — a `description` nested under another key describes
/// that key, not the command.
fn description(front: Option<&str>, prose: &str, name: &str) -> String {
    let declared = front
        .and_then(|yaml| crate::frontmatter::parse_tolerant(yaml).ok())
        .and_then(|parsed| {
            parsed
                .map
                .get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .map(str::to_owned)
        })
        .filter(|text| !text.is_empty());
    if let Some(declared) = declared {
        return declared;
    }
    for line in prose.lines() {
        let line = line.trim().trim_start_matches('#').trim();
        if !line.is_empty() {
            return line.to_owned();
        }
    }
    format!("command {name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn described(body: &str, name: &str) -> String {
        match crate::frontmatter::split(body) {
            Ok((front, prose)) => description(Some(front), prose, name),
            Err(_) => description(None, body, name),
        }
    }

    #[test]
    fn a_description_comes_from_frontmatter_then_prose_then_the_name() {
        assert_eq!(
            described(
                "---\ndescription: Ship it\nmodel: opus\n---\n\n# Ship\n",
                "ship"
            ),
            "Ship it"
        );
        assert_eq!(
            described("\n# Ship the branch\n\nSteps.\n", "ship"),
            "Ship the branch"
        );
        assert_eq!(described("---\nmodel: opus\n---\n", "ship"), "command ship");
        assert_eq!(described("", "ship"), "command ship");
    }

    /// A `description` indented under another key describes that key. Taking
    /// it as the command's own is how a scanner reads frontmatter; a parser
    /// sees the nesting.
    #[test]
    fn a_nested_description_never_beats_the_command_s_own() {
        let text = skill_text(
            "ship",
            "---\nallowed-tools:\n  description: NESTED WINS\ndescription: Ship the branch\n---\n\nBody.\n",
            "ship",
        );
        assert!(
            text.starts_with("---\nname: ship\ndescription: Ship the branch\n---\n"),
            "{text}"
        );
    }

    #[test]
    fn the_generated_skill_keeps_the_prose_and_drops_the_command_s_frontmatter() {
        let text = skill_text(
            "ship",
            "---\ndescription: do: it\nargument-hint: <branch>\n---\n\nBody.\n",
            "ship",
        );
        assert!(
            text.starts_with("---\nname: ship\ndescription: \"do: it\"\n---\n"),
            "{text}"
        );
        assert!(text.contains(GENERATED_BANNER));
        assert!(!text.contains("argument-hint"), "{text}");
        assert!(text.ends_with("Body.\n"));
    }
}
