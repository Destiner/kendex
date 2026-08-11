use std::path::PathBuf;
use std::process::Command;

use vstack_core::env::Env;
use vstack_core::lock::{Lock, load as load_lock, lock_path};
use vstack_core::model::{ItemKind, Scope};

use super::{CliResult, out, resolve_scopes, say};
use crate::scope::ScopeFilter;

pub const DEFAULT_UPSTREAM: &str = "vanillagreencom/vstack";

pub struct ReportArgs {
    pub skill: Option<String>,
    pub agent: Option<String>,
    pub hook: Option<String>,
    pub asset: Option<String>,
    pub title: String,
    pub body: Option<String>,
    pub body_file: Option<PathBuf>,
    pub global: bool,
    pub scope: Option<String>,
    pub upstream: Option<String>,
    pub area: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Area {
    Cli,
    Skills,
    Harness,
    ReviewGate,
    Docs,
    TechDebt,
}

impl Area {
    fn label(self) -> &'static str {
        match self {
            Area::Cli => "cli",
            Area::Skills => "skills",
            Area::Harness => "harness",
            Area::ReviewGate => "ci-infra",
            Area::Docs => "docs",
            Area::TechDebt => "chore",
        }
    }

    fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "cli" => Ok(Area::Cli),
            "skills" => Ok(Area::Skills),
            "harness" => Ok(Area::Harness),
            "review-gate" | "ci-infra" => Ok(Area::ReviewGate),
            "docs" => Ok(Area::Docs),
            "tech-debt" | "chore" => Ok(Area::TechDebt),
            other => Err(format!(
                "unknown --area '{other}'; expected one of: cli, skills, harness, review-gate, docs, tech-debt"
            )),
        }
    }

    fn derive(name: &str, kind: Option<ItemKind>) -> Self {
        if name.contains("review-gate") {
            return Area::ReviewGate;
        }
        match kind {
            Some(ItemKind::Hook | ItemKind::PiExtension) => Area::Harness,
            Some(ItemKind::Skill | ItemKind::Agent) => Area::Skills,
            _ => Area::Cli,
        }
    }
}

/// vstack-owned assets file upstream; everything else files against the
/// current repo — the safe default. Skills never route upstream via the
/// lock (distribution is not ownership); only their own frontmatter can
/// opt them in.
fn is_vstack_owned(
    lock: &Lock,
    name: &str,
    kind: Option<ItemKind>,
    frontmatter_source: Option<&str>,
    frontmatter_repo: Option<&str>,
    upstream: &str,
) -> bool {
    if frontmatter_source == Some("vstack") || frontmatter_repo == Some(DEFAULT_UPSTREAM) {
        return true;
    }
    lock.entries.values().any(|entry| {
        entry.name == name
            && kind.is_none_or(|k| k == entry.kind)
            && entry.kind != ItemKind::Skill
            && entry.source_repo == upstream
    })
}

fn installed_frontmatter(env: &Env, scope: &Scope, name: &str) -> (Option<String>, Option<String>) {
    let candidates = [match scope {
        Scope::Project { root } => root.join(".agents/skills").join(name).join("SKILL.md"),
        Scope::Global => env.rendered_skills_dir().join(name).join("SKILL.md"),
    }];
    for path in candidates {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(front) = text
            .strip_prefix("---")
            .and_then(|rest| rest.find("\n---").map(|end| rest[..end].to_owned()))
        else {
            continue;
        };
        let field = |key: &str| {
            front.lines().find_map(|line| {
                line.strip_prefix(key)
                    .map(|v| v.trim().trim_matches('"').to_owned())
                    .filter(|v| !v.is_empty())
            })
        };
        return (field("source:"), field("repository:"));
    }
    (None, None)
}

struct Inputs {
    selector: Option<(String, Option<ItemKind>)>,
    body: String,
    filter: ScopeFilter,
    area_override: Option<Area>,
    upstream: String,
}

fn parse_inputs(args: &ReportArgs) -> Result<Inputs, Box<dyn std::error::Error>> {
    let selectors = [
        (&args.skill, Some(ItemKind::Skill)),
        (&args.agent, Some(ItemKind::Agent)),
        (&args.hook, Some(ItemKind::Hook)),
        (&args.asset, None),
    ];
    let mut chosen: Vec<(String, Option<ItemKind>)> = selectors
        .iter()
        .filter_map(|(name, kind)| name.as_ref().map(|n| (n.clone(), *kind)))
        .collect();
    if chosen.len() > 1 {
        return Err("pass at most one of --skill, --agent, --hook, --asset".into());
    }
    let body = match (&args.body, &args.body_file) {
        (Some(_), Some(_)) => return Err("--body and --body-file are mutually exclusive".into()),
        (None, None) => return Err("provide --body or --body-file".into()),
        (Some(text), None) => text.clone(),
        (None, Some(path)) => std::fs::read_to_string(path)?,
    };
    let filter = ScopeFilter::resolve(args.scope.as_deref(), args.global, ScopeFilter::Project)?;
    if filter == ScopeFilter::All {
        return Err(
            "report resolves ownership against one lock; use --scope project or --scope global"
                .into(),
        );
    }
    Ok(Inputs {
        selector: chosen.pop(),
        body,
        filter,
        area_override: args.area.as_deref().map(Area::parse).transpose()?,
        upstream: args
            .upstream
            .clone()
            .unwrap_or_else(|| DEFAULT_UPSTREAM.to_owned()),
    })
}

pub fn run(env: &Env, args: ReportArgs) -> CliResult {
    let Inputs {
        selector,
        body,
        filter,
        area_override,
        upstream,
    } = parse_inputs(&args)?;

    let scope = resolve_scopes(env, filter)?.remove(0);
    let lock = load_lock(&lock_path(env, &scope))?;

    if selector.is_none() {
        say("warning: no asset selector — routing to this project's own repo");
    }
    let vstack_owned = selector.as_ref().is_some_and(|(name, kind)| {
        let (fm_source, fm_repo) = installed_frontmatter(env, &scope, name);
        is_vstack_owned(
            &lock,
            name,
            *kind,
            fm_source.as_deref(),
            fm_repo.as_deref(),
            &upstream,
        )
    });

    let mut gh_args = vec!["issue".to_owned(), "create".to_owned()];
    let mut sent_body = body.clone();
    let mut area = None;
    if vstack_owned {
        let (name, kind) = selector
            .as_ref()
            .map(|(n, k)| (n.as_str(), *k))
            .unwrap_or(("unknown", None));
        let kind_label = kind.map(ItemKind::name).unwrap_or("asset");
        sent_body.push_str(&format!(
            "\n\n<!-- vstack-report:v1 asset={name} kind={kind_label} ownership=vstack -->"
        ));
        gh_args.extend(["--repo".to_owned(), upstream.clone()]);
        // Routing labels exist only on the canonical repo; a fork override
        // must not carry one or gh fails with "label not found".
        if upstream == DEFAULT_UPSTREAM {
            let derived = area_override.unwrap_or_else(|| Area::derive(name, kind));
            gh_args.extend(["--label".to_owned(), derived.label().to_owned()]);
            area = Some(derived);
        }
    }
    gh_args.extend(["--title".to_owned(), args.title.clone()]);
    gh_args.extend(["--body".to_owned(), sent_body.clone()]);

    let ownership = if vstack_owned {
        "vstack"
    } else {
        "project-local"
    };
    if args.dry_run {
        say(&format!("ownership: {ownership}"));
        say(&format!(
            "target: {}",
            if vstack_owned {
                upstream.as_str()
            } else {
                "current repo origin"
            }
        ));
        if let Some(area) = area {
            say(&format!("label: {}", area.label()));
        }
        say(&format!("would run: gh {}", shell_join(&gh_args)));
        return Ok(());
    }

    let output = Command::new("gh").args(&gh_args).output();
    match output {
        Ok(result) if result.status.success() => {
            let url = String::from_utf8_lossy(&result.stdout).trim().to_owned();
            if url.is_empty() {
                out("Issue filed");
            } else {
                out(&format!("Issue filed: {url}"));
            }
            Ok(())
        }
        other => {
            let detail = match other {
                Ok(result) => String::from_utf8_lossy(&result.stderr).trim().to_owned(),
                Err(error) => error.to_string(),
            };
            let saved = save_body(&args.title, &sent_body);
            if let Some(path) = &saved {
                say(&format!("report body saved to {}", path.display()));
            }
            say("file it manually with the gh command above, or check `gh auth status`");
            Err(format!("failed to file the report via gh: {detail}").into())
        }
    }
}

fn save_body(title: &str, body: &str) -> Option<PathBuf> {
    let slug: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .take(40)
        .collect();
    let path = std::env::temp_dir().join(format!("vstack-report-{slug}-{}.md", std::process::id()));
    std::fs::write(&path, body).ok()?;
    Some(path)
}

fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|a| {
            if a.chars().any(|c| c.is_whitespace() || c == '"') {
                format!("{a:?}")
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
