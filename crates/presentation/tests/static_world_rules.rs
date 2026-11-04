//! Native map presentation observes the same resolved rules as the battle.
use presentation::static_world::files;
use serde_json::json;
use std::path::Path;

#[test]
fn native_ground_uses_encounter_sensors_and_forest_geometry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = files::rules(&root, "sensors")
        .unwrap()
        .overridden(json!({
            "sensors": {"fog_target_height_m": 3.25, "foliage_full_block": 2.5, "min_sight_gap_m": 0.75},
            "forests": {"rule": {"trunk_spacing_m": 17, "canopy_height_m": 11}}
        }).as_object().unwrap())
        .unwrap();
    let map = sim::maps::load_folder(&root.join("fixtures/maps/sensors")).unwrap();
    let battle_world = sim::world::WorldGeometry::new(&map.definition, &rules);
    let native = files::ground(&root, "sensors", &rules).unwrap();
    assert_eq!(
        native.sensors.fog_target_height_m,
        rules.sensors.fog_target_height_m
    );
    assert_eq!(
        native.sensors.foliage_full_block,
        rules.sensors.foliage_full_block
    );
    assert_eq!(
        native.sensors.min_sight_gap_m,
        rules.sensors.min_sight_gap_m
    );
    assert!(
        !native.source.foliage.is_empty(),
        "the map exercises forest geometry"
    );
    assert_eq!(native.source.foliage, battle_world.export_foliage());
    assert_eq!(native.source.props, battle_world.export_props());
}
