//! Safety decisions follow the item they are about. Removing the item
//! forgets them; renaming it carries them along. Either way no record is
//! left under a name it no longer describes, because an orphaned record is
//! what comes back to life the day something else takes that name.

use std::collections::BTreeMap;

use crate::model::ItemKind;

use super::Manifest;

impl Manifest {
    /// Forget every safety decision recorded for this item, whatever tool it
    /// was installed for. Removing an item is removing what the decisions
    /// were about; a record left behind would speak for a reinstall of the
    /// same name that nobody has looked at.
    pub fn reap_decisions(&mut self, kind: ItemKind, name: &str) {
        let about_item = |key: &str| {
            crate::lock::parse_entry_key(key)
                .is_some_and(|(key_kind, key_name, _)| key_kind == kind && key_name == name)
        };
        self.safety_overrides.retain(|key, _| !about_item(key));
        self.safety_reviews.retain(|key, _| !about_item(key));
    }

    /// Carry an item's safety decisions to its new name. The bytes and the
    /// findings are the same; only the key moved. Leaving the records under
    /// the old name would orphan them, and orphaned records are what come
    /// back to life the day something else takes that name.
    pub fn rename_decisions(&mut self, kind: ItemKind, old: &str, new: &str) {
        fn rekey<V>(table: &mut BTreeMap<String, V>, kind: ItemKind, old: &str, new: &str) {
            let moved: Vec<(String, String)> = table
                .keys()
                .filter_map(|key| {
                    let (key_kind, key_name, harness) = crate::lock::parse_entry_key(key)?;
                    (key_kind == kind && key_name == old)
                        .then(|| (key.clone(), crate::lock::entry_key(kind, new, harness)))
                })
                .collect();
            for (from, to) in moved {
                if let Some(record) = table.remove(&from) {
                    table.insert(to, record);
                }
            }
        }
        rekey(&mut self.safety_overrides, kind, old, new);
        rekey(&mut self.safety_reviews, kind, old, new);
    }
}
