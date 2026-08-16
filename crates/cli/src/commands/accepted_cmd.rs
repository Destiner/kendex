use clap::Args;
use vstack_core::apply;
use vstack_core::engine::ops::revoke_override;
use vstack_core::env::Env;
use vstack_core::manifest::{self, ManifestFile};

use super::{CliResult, resolve_scopes, say};
use crate::scope::ScopeFilter;

#[derive(Args)]
pub struct AcceptedArgs {
    /// Withdraw the acceptance stored under this key (kind:name:harness)
    #[arg(long)]
    revoke: Option<String>,
    #[arg(short = 'g', long)]
    global: bool,
    /// project | global (default project)
    #[arg(long)]
    scope: Option<String>,
}

/// The recorded acceptances of serious safety findings, and the way out of
/// one. Listing reads the manifests as they sit on disk; `--revoke` takes
/// the acceptance out by a planned, journaled write and moves nothing else
/// — the hold-back and the trash ride the next previewed apply.
pub fn run(env: &Env, args: AcceptedArgs) -> CliResult {
    let filter = ScopeFilter::resolve(args.scope.as_deref(), args.global, ScopeFilter::Project)?;
    for scope in resolve_scopes(env, filter)? {
        if let Some(key) = &args.revoke {
            let plan = revoke_override(env, &scope, key)?;
            apply::execute(env, &plan, None)?;
            say(&format!(
                "{}: withdrew the acceptance under '{key}' — the item is held back again; the next apply moves its installed copy to the trash",
                scope.label()
            ));
            continue;
        }
        let path = manifest::manifest_path(env, &scope);
        let ManifestFile::Current(manifest) = manifest::load(&path)? else {
            say(&format!("{}: no acceptances", scope.label()));
            continue;
        };
        if manifest.safety_overrides.is_empty() {
            say(&format!("{}: no acceptances", scope.label()));
            continue;
        }
        say(&format!("{}:", scope.label()));
        for (key, recorded) in &manifest.safety_overrides {
            say(&format!(
                "  {key} — {} finding{} accepted {}",
                recorded.findings.len(),
                if recorded.findings.len() == 1 {
                    ""
                } else {
                    "s"
                },
                recorded.granted_at
            ));
        }
    }
    Ok(())
}
