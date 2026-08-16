mod commands;
mod flags;
mod scope;

use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vstack_core::env::Env;

use commands::project::ProjectCommand;
use flags::{AddFlags, ReportFlags};
use scope::ScopeFilter;

#[derive(Parser)]
#[command(
    name = "vstack",
    version,
    about = "Skills, agents, hooks. Cross-harness."
)]
struct Cli {
    /// Bare form: `vstack <source> [flags]` maps to `add`.
    source: Option<String>,
    #[command(flatten)]
    add_flags: AddFlags,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Install agents, skills, and more from a source
    Add {
        /// GitHub `owner/repo` or local path
        source: Option<String>,
        #[command(flatten)]
        flags: AddFlags,
    },
    /// Hold an item at a version, or let it follow its source again
    Pin(commands::pin::PinArgs),
    /// Remove installed items
    Remove {
        names: Vec<String>,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default project)
        #[arg(long)]
        scope: Option<String>,
        /// Also remove what nothing needs anymore
        #[arg(long)]
        sweep: bool,
        /// Keep what nothing needs anymore
        #[arg(long, conflicts_with = "sweep")]
        no_sweep: bool,
    },
    /// Regenerate every declared installation from its source
    Refresh {
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
        /// Per-item detail instead of the compact summary
        #[arg(short = 'v', long)]
        verbose: bool,
        /// Accept changes to what is installed without asking
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Check installs against the lock; non-zero exit on drift
    Verify {
        names: Vec<String>,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
    },
    /// Make disk match declaration, orphan cleanup included
    Apply {
        /// Print the plan and change nothing
        #[arg(long)]
        plan: bool,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default project)
        #[arg(long)]
        scope: Option<String>,
        #[arg(short = 'y', long)]
        yes: bool,
        /// Install an item despite its safety findings, as `name@hash` using
        /// the hash printed beside them — a bare name does not grant
        #[arg(long = "allow-unsafe")]
        allow_unsafe: Vec<String>,
    },
    /// Record an observed item into the manifest (content moves to the
    /// local source)
    Adopt {
        /// agent | skill
        kind: String,
        name: String,
        #[arg(long)]
        harness: Option<String>,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global (default project)
        #[arg(long)]
        scope: Option<String>,
    },
    /// Register, list, and discover vstack-enabled projects
    #[command(subcommand)]
    Project(ProjectCommand),
    /// List everything observed on this machine
    #[command(alias = "ls")]
    List {
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
        /// Filter by harness id
        #[arg(long)]
        harness: Option<String>,
    },
    /// Detection + declaration sanity for this machine, or authoring
    /// validation over a catalog directory with --catalog
    Check {
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
        /// Validate this catalog directory instead of this machine
        #[arg(long)]
        catalog: Option<std::path::PathBuf>,
        /// With --catalog, also fail on advisories
        #[arg(long)]
        strict: bool,
    },
    /// File an issue about an installed asset, routed by ownership
    Report(ReportFlags),
    /// Migrate v1 manifests and locks to v2 (originals go to the trash)
    Import {
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
    },
    /// Declare, toggle, and refresh sources
    #[command(subcommand)]
    Source(commands::source_cmd::SourceCommand),
    /// Scaffold a new catalog item in the current directory
    Init {
        name: Option<String>,
        /// agent | skill | hook
        #[arg(long)]
        kind: Option<String>,
    },
    /// Self-update from the release feed
    Update {
        /// Reinstall even when the version matches
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Update Pi extension packages
    #[command(name = "update-pi")]
    UpdatePi {
        /// Print the plan and change nothing
        #[arg(short = 'c', long)]
        check: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
    },
}

/// Sanity for this machine, or for a catalog directory. They answer
/// different questions — what is installed here, versus what this content
/// would do anywhere — and only the second one belongs in a repository's CI.
fn check(
    env: &Env,
    global: bool,
    scope: Option<String>,
    catalog: Option<std::path::PathBuf>,
    strict: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match catalog {
        Some(catalog) => commands::check_catalog::run(&catalog, strict),
        None => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            commands::check::run(env, filter)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let env = Env::detect()?;
    let command = match cli.command {
        Some(command) => command,
        None => {
            if cli.source.is_none() {
                return Err("nothing to do — pass a source to add, or a subcommand".into());
            }
            commands::add::run(&env, cli.add_flags.into_args(cli.source))?;
            return Ok(ExitCode::SUCCESS);
        }
    };
    match command {
        Command::Add { source, flags } => commands::add::run(&env, flags.into_args(source))?,
        Command::Pin(args) => commands::pin::run(&env, args)?,
        Command::Remove {
            names,
            global,
            scope,
            sweep,
            no_sweep,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::Project)?;
            let sweep = match (sweep, no_sweep) {
                (true, _) => Some(true),
                (_, true) => Some(false),
                _ => None,
            };
            commands::remove::run(&env, names, filter, sweep)?;
        }
        Command::Refresh {
            global,
            scope,
            verbose,
            yes,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            commands::refresh::run(&env, filter, verbose, yes)?;
        }
        Command::Verify {
            names,
            global,
            scope,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            return commands::verify::run(&env, names, filter);
        }
        Command::Apply {
            plan,
            global,
            scope,
            yes,
            allow_unsafe,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::Project)?;
            commands::apply_cmd::run(&env, filter, plan, yes, allow_unsafe)?;
        }
        Command::Adopt {
            kind,
            name,
            harness,
            global,
            scope,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::Project)?;
            commands::adopt::run(&env, kind, name, harness, filter)?;
        }
        Command::Project(cmd) => commands::project::run(&env, cmd)?,
        Command::List {
            global,
            scope,
            harness,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            commands::list::run(&env, filter, harness)?;
        }
        Command::Check {
            global,
            scope,
            catalog,
            strict,
        } => check(&env, global, scope, catalog, strict)?,
        Command::UpdatePi { check, scope } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), false, ScopeFilter::All)?;
            commands::update_pi::run(&env, filter, check)?;
        }
        Command::Report(flags) => commands::report::run(&env, flags.into_args())?,
        Command::Import { global, scope } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            commands::import::run(&env, filter)?;
        }
        Command::Source(source_command) => {
            let filter = ScopeFilter::resolve(None, false, ScopeFilter::Project)?;
            commands::source_cmd::run(&env, source_command, filter)?;
        }
        Command::Init { name, kind } => commands::init::run(name, kind)?,
        Command::Update { force } => commands::update::run(force)?,
    }
    Ok(ExitCode::SUCCESS)
}
