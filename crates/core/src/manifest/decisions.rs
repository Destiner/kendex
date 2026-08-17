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
            crate::lock::parse_entry_key(key).is_some_and(|(key_kind, key_name, _)| {
                key_kind == kind && names(kind, key_name, name)
            })
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
                    (key_kind == kind && names(kind, key_name, old)).then(|| {
                        let renamed = match key_name.rsplit_once(':') {
                            Some((registration, _)) if kind == ItemKind::Hook => {
                                format!("{registration}:{new}")
                            }
                            _ => new.to_owned(),
                        };
                        (key.clone(), crate::lock::entry_key(kind, &renamed, harness))
                    })
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

/// Whether a recorded key's name is this item's. A hook is declared by one
/// name and observed by another — the scanner names it after its
/// registration, `event:matcher:stem`, where the stem is the script the
/// declaration wrote — so a hook's records live under both spellings and
/// both belong to it.
fn names(kind: ItemKind, key_name: &str, name: &str) -> bool {
    key_name == name
        || (kind == ItemKind::Hook
            && key_name
                .rsplit_once(':')
                .is_some_and(|(_, stem)| stem == name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::reviews::SafetyReview;

    fn with(keys: &[&str]) -> Manifest {
        let mut manifest = Manifest::default();
        for key in keys {
            manifest
                .safety_reviews
                .insert((*key).to_owned(), SafetyReview::of("h"));
        }
        manifest
    }

    /// A hook's records live under the declared name and under the
    /// registration the scanner named it by; removing the hook clears both.
    #[test]
    fn a_hooks_observed_spelling_is_reaped_with_its_declared_one() {
        let mut manifest = with(&[
            "hook:guard:claude",
            "hook:PreToolUse:Bash:guard:claude",
            "hook:PreToolUse:Bash:other:claude",
            "skill:guard:claude",
        ]);
        manifest.reap_decisions(ItemKind::Hook, "guard");
        let left: Vec<&String> = manifest.safety_reviews.keys().collect();
        assert_eq!(
            left,
            ["hook:PreToolUse:Bash:other:claude", "skill:guard:claude"]
        );
    }

    #[test]
    fn a_rename_carries_both_spellings_to_the_new_name() {
        let mut manifest = with(&["hook:guard:claude", "hook:PreToolUse:Bash:guard:claude"]);
        manifest.rename_decisions(ItemKind::Hook, "guard", "gate");
        let left: Vec<&String> = manifest.safety_reviews.keys().collect();
        assert_eq!(
            left,
            ["hook:PreToolUse:Bash:gate:claude", "hook:gate:claude"]
        );
    }
}
