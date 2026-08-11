mod commands;
mod scope;

use std::io::Write;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use vstack_core::env::Env;

use commands::add::AddArgs;
use commands::project::ProjectCommand;
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

/// The v1 add surface — shared by `add` and the bare form, flag for flag.
#[derive(Args)]
struct AddFlags {
    /// Install to the user-level scope
    #[arg(short = 'g', long)]
    global: bool,
    /// Target harnesses (comma-separated)
    #[arg(long)]
    harness: Vec<String>,
    /// Install specific agents (comma-separated)
    #[arg(short = 'a', long)]
    agent: Vec<String>,
    /// Install specific skills (comma-separated)
    #[arg(short = 's', long)]
    skill: Vec<String>,
    /// Install specific hooks (comma-separated)
    #[arg(long)]
    hook: Vec<String>,
    /// Install specific Pi extensions (comma-separated)
    #[arg(long, visible_alias = "pi-package")]
    pi_extension: Vec<String>,
    /// Copy instead of symlink
    #[arg(long)]
    copy: bool,
    /// Skip confirmation prompts
    #[arg(short = 'y', long)]
    yes: bool,
    /// All items to all harnesses
    #[arg(long)]
    all: bool,
    /// Allow --global --all over a non-empty global lock
    #[arg(long)]
    clobber: bool,
    /// Skip auto-install of skills referenced by selected agents
    #[arg(long)]
    no_auto_skills: bool,
}

impl AddFlags {
    fn into_args(self, source: Option<String>) -> AddArgs {
        AddArgs {
            source,
            global: self.global,
            harness: self.harness,
            agent: self.agent,
            skill: self.skill,
            hook: self.hook,
            pi_extension: self.pi_extension,
            copy: self.copy,
            yes: self.yes,
            all: self.all,
            clobber: self.clobber,
            no_auto_skills: self.no_auto_skills,
        }
    }
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
    /// Remove installed items
    Remove {
        names: Vec<String>,
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default project)
        #[arg(long)]
        scope: Option<String>,
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
    /// Detection + declaration sanity for this machine
    Check {
        #[arg(short = 'g', long)]
        global: bool,
        /// project | global | all (default all)
        #[arg(long)]
        scope: Option<String>,
    },
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
        Command::Remove {
            names,
            global,
            scope,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::Project)?;
            commands::remove::run(&env, names, filter)?;
        }
        Command::Refresh {
            global,
            scope,
            verbose,
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            commands::refresh::run(&env, filter, verbose)?;
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
        } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::Project)?;
            commands::apply_cmd::run(&env, filter, plan, yes)?;
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
        Command::Check { global, scope } => {
            let filter = ScopeFilter::resolve(scope.as_deref(), global, ScopeFilter::All)?;
            commands::check::run(&env, filter)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
