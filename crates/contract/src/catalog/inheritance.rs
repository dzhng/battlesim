//! Authored parent resolution, shared by every catalog section.
use super::{merge_entry, CatalogError};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Flatten `extends` chains; abstract entries are dropped.
pub fn inherit(
    section: &'static str,
    entries: &Map<String, Value>,
) -> Result<BTreeMap<String, Value>, CatalogError> {
    fn flat(
        section: &'static str,
        id: &str,
        entries: &Map<String, Value>,
        done: &mut BTreeMap<String, Value>,
        chain: &mut Vec<String>,
    ) -> Result<Value, CatalogError> {
        if let Some(v) = done.get(id) {
            return Ok(v.clone());
        }
        if chain.iter().any(|c| c == id) {
            chain.push(id.to_string());
            return Err(CatalogError::Cycle {
                section,
                chain: chain.clone(),
            });
        }
        let mut own = entries[id].clone();
        let own_map = own
            .as_object_mut()
            .ok_or_else(|| CatalogError::NotAnObject(format!("{section}.{id}")))?;
        let parent = own_map.remove("extends");
        let resolved = match parent {
            None => own,
            Some(Value::String(parent)) => {
                if !entries.contains_key(&parent) {
                    return Err(CatalogError::UnknownParent {
                        section,
                        id: id.to_string(),
                        parent,
                    });
                }
                chain.push(id.to_string());
                let mut base = flat(section, &parent, entries, done, chain)?;
                chain.pop();
                base.as_object_mut()
                    .expect("resolved entries are objects")
                    .remove("abstract");
                merge_entry(section, &mut base, &own);
                base
            }
            Some(other) => {
                return Err(CatalogError::Invalid {
                    section,
                    id: id.to_string(),
                    error: format!("extends must be an id, not {other}"),
                })
            }
        };
        done.insert(id.to_string(), resolved.clone());
        Ok(resolved)
    }
    let mut done = BTreeMap::new();
    for id in entries.keys() {
        flat(section, id, entries, &mut done, &mut Vec::new())?;
    }
    Ok(done
        .into_iter()
        .filter(|(_, v)| v.get("abstract") != Some(&Value::Bool(true)))
        .map(|(id, mut v)| {
            v.as_object_mut()
                .expect("resolved entries are objects")
                .remove("abstract");
            (id, v)
        })
        .collect())
}
