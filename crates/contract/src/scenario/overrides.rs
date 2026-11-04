//! Re-admit encounter patches through the authored rule contract.
use super::Rules;

impl Rules {
    /// These rules with a scenario's `overrides` merged in, read and checked
    /// again as the shipped rules are, so an override is held to every rule
    /// a shipped row is. The patch is shaped like the authored rules: a rules
    /// section of `rules.json` (`weapons`, `sensors`, `air`, ...) or a catalog
    /// section (`units`, `soldiers`, `parts`, `roles`, `props`). It merges
    /// into the resolved rows as an `extends` child merges into its parent
    /// ([`crate::catalog::merge_entry`]); a row the rules lack is added, and
    /// may `extends` any resolved row of its section.
    pub fn overridden(
        self,
        overrides: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Rules, String> {
        use crate::catalog::{merge, merge_entry, SECTIONS};
        if overrides.is_empty() {
            return Ok(self);
        }
        let mut document = serde_json::to_value(&self).map_err(|e| e.to_string())?;
        // A resolved range is output, not an authored constraint on a new
        // layer override. Recompute it after merging the authored patch.
        for row in document["weapons"].as_object_mut().unwrap().values_mut() {
            row.as_object_mut().unwrap().remove("range_m");
        }
        for (section, patch) in overrides {
            let in_catalog = SECTIONS.contains(&section.as_str());
            let base = match in_catalog {
                // A resolved catalog travels as one document.
                true => document["catalog"][0].get_mut(section),
                false if section == "catalog" => None,
                false => document.get_mut(section),
            }
            .ok_or_else(|| format!("{section} is not a section of the rules or the catalog"))?;
            if !in_catalog {
                merge(base, patch);
                continue;
            }
            let (Some(rows), Some(patch)) = (base.as_object_mut(), patch.as_object()) else {
                return Err(format!("{section} is a table of rows by id"));
            };
            for (id, row) in patch {
                match rows.get_mut(id) {
                    Some(base) => merge_entry(section, base, row),
                    None => {
                        rows.insert(id.clone(), row.clone());
                    }
                }
            }
        }
        serde_json::from_value(document).map_err(|e| e.to_string())
    }
}
