//! Authored parent resolution and the merge rules it applies, shared by
//! every catalog section.
use super::CatalogError;
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

/// A unit type's components written as a one-key variant object
/// (`{ "hull": … }`, `{ "tracked": … }`).
const VARIANTS: [&str; 2] = ["body", "mobility"];

/// Merge an `extends` child (or a part's patch) into an entry of `section`,
/// as [`merge`] does, with two rules for unit types: a component the child
/// writes as another variant (`mobility: { wheeled }` over `tracked`)
/// replaces the parent's rather than merging both, and `parts` gather along
/// the chain, the parent's first. Idempotent.
pub fn merge_entry(section: &str, base: &mut Value, over: &Value) {
    let parts = match (section, base.as_object_mut(), over.as_object()) {
        ("units", Some(b), Some(o)) => {
            for key in VARIANTS {
                let swapped = match (b.get(key).and_then(variant), o.get(key).and_then(variant)) {
                    (Some(was), Some(now)) => was != now,
                    _ => false,
                };
                if swapped {
                    b.remove(key);
                }
            }
            let listed = |v: Option<&Value>| -> Vec<Value> {
                v.and_then(Value::as_array).cloned().unwrap_or_default()
            };
            let mut parts = listed(b.get("parts"));
            for p in listed(o.get("parts")) {
                if !parts.contains(&p) {
                    parts.push(p);
                }
            }
            Some(parts).filter(|p| !p.is_empty())
        }
        _ => None,
    };
    merge(base, over);
    if let Some(parts) = parts {
        base["parts"] = Value::Array(parts);
    }
}

/// The variant a one-key object names.
fn variant(v: &Value) -> Option<&str> {
    let o = v.as_object()?;
    (o.len() == 1).then(|| o.keys().next().map(String::as_str))?
}

/// Merge `over` into `base`: objects key by key, lists of objects with ids
/// by id (a new id is appended), anything else replaced. Idempotent.
pub fn merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(slot) => merge(slot, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (Value::Array(b), Value::Array(o)) if keyed(b) && keyed(o) => {
            for item in o {
                let id = &item["id"];
                match b.iter_mut().find(|x| &x["id"] == id) {
                    Some(slot) => merge(slot, item),
                    None => b.push(item.clone()),
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}

/// A non-empty list whose every item is an object with a string `id`.
fn keyed(list: &[Value]) -> bool {
    !list.is_empty()
        && list
            .iter()
            .all(|x| x.get("id").is_some_and(Value::is_string))
}
