//! Authored parent resolution, shared by every catalog section.
use super::{merge_entry, CatalogError};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

struct Resolved {
    row: Value,
    ancestors: Vec<String>,
}

fn flat(
    section: &'static str,
    id: &str,
    entries: &Map<String, Value>,
    done: &mut BTreeMap<String, Resolved>,
    chain: &mut Vec<String>,
) -> Result<Value, CatalogError> {
    if let Some(v) = done.get(id) {
        return Ok(v.row.clone());
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
    let mut ancestors = Vec::new();
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
            ancestors.push(parent.clone());
            ancestors.extend(done[&parent].ancestors.iter().cloned());
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
    done.insert(
        id.to_string(),
        Resolved {
            row: resolved.clone(),
            ancestors,
        },
    );
    Ok(resolved)
}

fn resolve(
    section: &'static str,
    entries: &Map<String, Value>,
) -> Result<BTreeMap<String, Resolved>, CatalogError> {
    let mut done = BTreeMap::new();
    for id in entries.keys() {
        flat(section, id, entries, &mut done, &mut Vec::new())?;
    }
    Ok(done)
}

/// Authored ancestors, nearest first, including abstract parents. Authoring
/// tools use these to identify field sources without resolving parents again.
pub fn ancestry(
    section: &'static str,
    entries: &Map<String, Value>,
) -> Result<BTreeMap<String, Vec<String>>, CatalogError> {
    Ok(resolve(section, entries)?
        .into_iter()
        .map(|(id, resolved)| (id, resolved.ancestors))
        .collect())
}

/// Flatten `extends` chains; abstract entries are dropped.
pub fn inherit(
    section: &'static str,
    entries: &Map<String, Value>,
) -> Result<BTreeMap<String, Value>, CatalogError> {
    Ok(resolve(section, entries)?
        .into_iter()
        .filter(|(_, resolved)| resolved.row.get("abstract") != Some(&Value::Bool(true)))
        .map(|(id, mut resolved)| {
            resolved
                .row
                .as_object_mut()
                .expect("resolved entries are objects")
                .remove("abstract");
            (id, resolved.row)
        })
        .collect())
}
