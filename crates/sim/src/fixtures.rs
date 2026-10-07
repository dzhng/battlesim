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
    documents(&["units", "props"])
}

/// The catalog documents a test of a mechanic runs on: the stand-in units
/// (`fixtures/units/generic`), the roles and the props, never the faction
/// roster. A unit added to the roster then changes no test; a test that needs
/// a shape the stand-ins lack adds its own fake unit.
pub fn stand_in_documents() -> Vec<Value> {
    documents(&["units/generic", "units/roles.json", "props"])
}

/// The game fixture as [`game`] reads it, with [`stand_in_documents`] as its
/// catalog.
pub fn stand_in_game() -> Value {
    let mut fixture = game();
    fixture["catalog"] = Value::Array(stand_in_documents());
    fixture
}

/// Add to `fixture`'s catalog a hull at each drive's absolute limits
/// (`hull_limits`): `limit_tracked`, the stand-in tank grown to the tracked
/// limits, and `limit_wheeled`, the stand-in truck grown to the wheeled
/// limits and turning as wide as they allow. What is proven for them holds
/// for every unit the limits admit.
pub fn with_units_at_limits(fixture: &mut Value) {
    let limits = fixture["hull_limits"].clone();
    let hull = |base: &str, drive: &str| {
        let height = fixture["catalog"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|d| d["units"][base]["body"]["hull"]["half_extents_m"][2].as_f64())
            .unwrap_or_else(|| panic!("the stand-in {base} has a hull"));
        serde_json::json!({ "hull": { "half_extents_m": [
            limits[drive]["half_length_m"], limits[drive]["half_width_m"], height
        ] } })
    };
    let units = serde_json::json!({
        "limit_tracked": { "extends": "tank", "name": "Tracked hull at the limits", "body": hull("tank", "tracked") },
        "limit_wheeled": {
            "extends": "supply", "name": "Wheeled hull at the limits", "body": hull("supply", "wheeled"),
            "mobility": { "wheeled": { "turning_radius_m": limits["wheeled"]["turning_radius_m"] } }
        },
    });
    fixture["catalog"]
        .as_array_mut()
        .expect("a fixture with its catalog")
        .push(serde_json::json!({ "units": units }));
}

/// Lift `fixture`'s hull limits, for a test that needs a hull no battle may
/// field (a truck too long to turn a corner, a tank wider than any street)
/// to show how a mechanic fails.
pub fn lift_hull_limits(fixture: &mut Value) {
    let far =
        serde_json::json!({ "half_width_m": 1e6, "half_length_m": 1e6, "turning_radius_m": 1e6 });
    fixture["hull_limits"] = serde_json::json!({ "tracked": far, "wheeled": far });
}

/// The catalog documents at `roots` under `fixtures/` (a directory, walked,
/// or one file), in path order.
fn documents(roots: &[&str]) -> Vec<Value> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("a catalog directory is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "json")
                && path
                    .file_name()
                    .is_none_or(|name| name != "model-manifest.json")
            {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    for root in roots {
        let root = dir().join(root);
        if root.is_dir() {
            walk(&root, &mut paths);
        } else {
            paths.push(root);
        }
    }
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

/// Validate supplied mechanics and return the resolved catalog used by the browser.
/// This does not load maps or write fixture files.
pub fn admit(mut game: Value, documents: Vec<Value>) -> Result<Value, String> {
    if !game.is_object() {
        return Err("game must be an object".into());
    }
    game["catalog"] = Value::Array(documents);
    let rules: contract::scenario::Rules =
        serde_json::from_value(game).map_err(|e| e.to_string())?;
    crate::weapons::check_rules(&rules)?;
    crate::supply::validate(&rules)?;
    let mut view = rules.catalog.view();
    view["weapons"] = serde_json::to_value(rules.weapons).map_err(|e| e.to_string())?;
    Ok(view)
}

/// The resolved catalog's view (`Catalog::view`) as `fixtures/catalog.json`
/// holds it, with `weapons`: `game.json`'s weapon rows, `extends`
/// resolved, as the mounts name them.
pub fn catalog_view() -> String {
    let view = admit(read(&dir().join("game.json")), catalog_documents())
        .unwrap_or_else(|e| panic!("the mechanics: {e}"));
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
