use crate::engine::EngineReport;
use crate::env::Env;
use crate::error::Result;
use crate::manifest::Manifest;
use crate::model::Scope;

/// The plan must persist the mutated manifest exactly once; plan_scope adds
/// its own write only when upstream skill merges changed it further.
pub(super) fn ensure_manifest_persisted(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    report: &mut EngineReport,
) -> Result<()> {
    let already = crate::engine::persists_manifest(&report.plan.ops);
    if already {
        return Ok(());
    }
    crate::rename::insert_manifest_save(env, scope, &mut report.plan, manifest.clone())
}
