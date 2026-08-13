use crate::engine::EngineReport;
use crate::env::Env;
use crate::error::Result;
use crate::manifest::{self, Manifest};
use crate::model::Scope;

/// The plan must persist the mutated manifest exactly once; plan_scope adds
/// its own write only when upstream skill merges changed it further.
pub(super) fn ensure_manifest_persisted(
    env: &Env,
    scope: &Scope,
    manifest: &Manifest,
    report: &mut EngineReport,
) -> Result<()> {
    let already = report
        .plan
        .ops
        .iter()
        .any(|op| matches!(op.op, crate::apply::Op::WriteManifest { .. }));
    if already {
        return Ok(());
    }
    let path = manifest::manifest_path(env, scope);
    report.plan.ops.insert(
        0,
        crate::apply::PlannedOp {
            description: "Save vstack.toml".into(),
            op: crate::apply::Op::WriteManifest {
                pre: crate::apply::Pre::observed(&path)?,
                path,
                manifest: Box::new(manifest.clone()),
            },
        },
    );
    Ok(())
}
