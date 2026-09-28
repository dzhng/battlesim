//! The shipped fixtures as native tools (tests, reports, the shot runner)
//! read them: `fixtures/village.json` with the unit catalog's documents,
//! every `fixtures/units/**/*.json`, as its `catalog`. The browser gets the
//! same catalog, resolved, from `fixtures/unit-catalog.json`, which
//! [`catalog_view`] writes and a test keeps current.
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The repository's `fixtures/`.
pub fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn read(path: &Path) -> Value {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

/// Every catalog document under `fixtures/units/`, in path order.
pub fn catalog_documents() -> Vec<Value> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("fixtures/units is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "json") {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    walk(&dir().join("units"), &mut paths);
    paths.sort();
    paths.iter().map(|p| read(p)).collect()
}

/// The village fixture with the unit catalog: what `Rules` and the village
/// and endurance scenarios read.
pub fn village() -> Value {
    let mut fixture = read(&dir().join("village.json"));
    fixture["catalog"] = Value::Array(catalog_documents());
    fixture
}

/// The resolved catalog's view (`Catalog::view`) as `fixtures/unit-catalog.json` holds it.
pub fn catalog_view() -> String {
    let catalog = contract::catalog::resolve(&catalog_documents())
        .unwrap_or_else(|e| panic!("the unit catalog: {e}"));
    serde_json::to_string_pretty(&catalog.view()).expect("the view serializes") + "\n"
}

/// Merge `patch` into the catalog entry `section.id` of a fixture's
/// documents (`fixture["catalog"]`), as an `extends` child would: how tests
/// and scenarios tune one type.
pub fn patch_catalog(fixture: &mut Value, section: &str, id: &str, patch: Value) {
    let doc = fixture["catalog"]
        .as_array_mut()
        .expect("a fixture with its catalog")
        .iter_mut()
        .find(|d| d[section].get(id).is_some())
        .unwrap_or_else(|| panic!("no {section}.{id} in the catalog"));
    contract::catalog::merge(&mut doc[section][id], &patch);
}
