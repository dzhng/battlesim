//! The replay places the map's props from the art a page running the
//! scene's units installs: a unit's wreck is drawn as that unit's own, and a
//! region's looks only on a map of that region. Reads the fixture run's
//! world and the props' bundles (Git LFS).
use std::path::Path;

use native_client::bundle::Library;
use native_client::capture::{Replay, ROOT};
use presentation::assets::RUNTIME;
use presentation::static_world::files;
use presentation::static_world::props::{DRAWS_PROP, DRAWS_PROP_GROUND};
use serde_json::Value;

const RUN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/run/paris-corner"
);

/// The names of the world's prop rows.
fn prop_rows(replay: &Replay) -> Vec<String> {
    let world = replay.presentation().world();
    let text = |at: u32, len: u32| world.strings[at as usize..(at + len) as usize].to_owned();
    world
        .appearances
        .iter()
        .filter(|a| a.draws == DRAWS_PROP || a.draws == DRAWS_PROP_GROUND)
        .map(|a| text(a.name, a.name_len))
        .collect()
}

#[test]
fn the_scenes_vehicles_leave_their_own_wrecks_and_the_map_its_own_regions_looks() {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let replay = Replay::open(Path::new(RUN), &library).unwrap();
    let rows = prop_rows(&replay);
    assert!(!rows.is_empty(), "the map's props are placed");
    // The scene's tank is a capture unit: its wreck is the hull's own.
    assert!(
        rows.iter().any(|r| r == "us_m1_abrams_sep_v2_wreck"),
        "the tank's own wreck is among the props' looks: {rows:?}"
    );
    let catalog: Value = serde_json::from_str(
        &std::fs::read_to_string(Path::new(RUNTIME).join("catalog.json")).unwrap(),
    )
    .unwrap();
    let family = |name: &str| catalog["appearances"][name]["regional_family"].as_str();
    let regional: Vec<_> = rows.iter().filter_map(|r| family(r)).collect();
    assert!(!regional.is_empty(), "paris-corner draws Paris's own looks");
    assert!(
        regional.iter().all(|&f| f == "paris"),
        "only Paris's looks: {regional:?}"
    );
}

#[test]
fn a_capture_maps_props_are_placed_under_the_capture_units() {
    let root = Path::new(ROOT);
    let rules = files::rules(root, "paris-corner").unwrap();
    let ground = files::ground(root, "paris-corner", &rules).unwrap();
    let layout: Value = serde_json::from_str(&ground.layout).unwrap();
    // The scene's tank is a capture unit: its wreck is drawn as its own hull's.
    assert_eq!(
        layout["unitAppearance"]["capture_us_tank"], "us_m1_abrams_sep_v2",
        "the layout names the capture units"
    );
}
