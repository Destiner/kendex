use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::configedit::ConfigEdit;

/// Every structured edit the plan wants, grouped by config file. The plan
/// composes each file's edits into one mutation with one precondition —
/// per-edit preconditions against the same original bytes can never all
/// hold once the first edit lands.
#[derive(Debug, Default)]
pub(super) struct ConfigEditPlan {
    pub(super) by_file: BTreeMap<PathBuf, (Vec<String>, Vec<ConfigEdit>)>,
}

impl ConfigEditPlan {
    pub(super) fn push(&mut self, path: PathBuf, label: String, edit: ConfigEdit) {
        let entry = self.by_file.entry(path).or_default();
        entry.0.push(label);
        entry.1.push(edit);
    }
}
