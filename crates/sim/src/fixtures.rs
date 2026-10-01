//! The shipped fixtures as native tools (tests, reports, the shot runner)
//! read them: `fixtures/game.json` with the catalog's documents, every
//! `fixtures/units/**/*.json` and `fixtures/props/**/*.json`, as its
//! `catalog`. The browser gets the same catalog, resolved, from
//! `fixtures/catalog.json`, which [`catalog_view`] writes and a test
//! keeps current. Maps are read by id through [`crate::maps`].
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

/// Every catalog document under `fixtures/units/` and `fixtures/props/`, in
/// path order.
pub fn catalog_documents() -> Vec<Value> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("a catalog directory is readable") {
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
    walk(&dir().join("props"), &mut paths);
    paths.sort();
    paths
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(p)
                .unwrap_or_else(|e| panic!("reading {}: {e}", p.display()));
            // A key written twice in one file is refused, not silently
            // dropped for the second.
            contract::catalog::parse_document(&text)
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        })
        .collect()
}

/// The game fixture as `Rules` and the village scenario read it: the rules
/// (`game.json`), the unit catalog, and the village's map, resolved from
/// the saved catalogue (`fixtures/maps/village`), under `map`.
pub fn game() -> Value {
    let mut fixture = read(&dir().join("game.json"));
    fixture["catalog"] = Value::Array(catalog_documents());
    let map = crate::maps::load("village").unwrap_or_else(|e| panic!("the village's map: {e}"));
    fixture["map"] = serde_json::to_value(map.definition).expect("the map serializes");
    fixture
}

/// The resolved catalog's view (`Catalog::view`) as `fixtures/catalog.json`
/// holds it, with `weapons`: `game.json`'s weapon rows, `extends`
/// resolved, as the mounts name them.
pub fn catalog_view() -> String {
    let catalog = contract::catalog::resolve(&catalog_documents())
        .unwrap_or_else(|e| panic!("the unit catalog: {e}"));
    let rows = read(&dir().join("game.json"))["weapons"].take();
    let weapons =
        contract::weapons::resolve_weapons(rows).unwrap_or_else(|e| panic!("the weapon rows: {e}"));
    let mut view = catalog.view();
    view["weapons"] = serde_json::to_value(weapons).expect("the rows serialize");
    serde_json::to_string_pretty(&view).expect("the view serializes") + "\n"
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
    contract::catalog::merge_entry(section, &mut doc[section][id], &patch);
}
