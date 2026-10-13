//! Radar signatures are a separate, SEAD-only observation channel.

use contract::ids::Side;
use contract::observation::ContactSource;
use serde_json::json;
use sim::battle::Battle;

fn battle() -> Battle {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air" } },
            "capabilities": { "air": {
                "fuel_capacity_s": 180.0,
                "sead": true,
                "radar": { "emits": false, "detection_m": 1200.0 },
                "refund": { "health": 0.34, "ammunition": 0.33, "fuel": 0.33 }
            }}
        }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({
            "capabilities": {
                "air_defense": { "tier": "long" },
                "radar": { "emits": true, "detection_m": 1200.0 }
            }
        }),
    );
    let map = json!({
        "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35
    });
    let setup = serde_json::from_value(json!({
        "map": map,
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_heli", "position": [100, 200] },
            { "side": "red", "kind": "test_jeep", "position": [500, 200] }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

#[test]
fn only_a_high_air_radar_detector_gets_approximate_radar_contact() {
    let b = battle();
    let blue = b.observe(Side::Blue);
    let [contact] = blue.contacts.as_slice() else {
        panic!("expected one radar contact: {:?}", blue.contacts);
    };
    assert_eq!(contact.source, ContactSource::Radar);
    assert!(contact.kind.is_none(), "radar never identifies its emitter");
    assert!(contact.bearing_deg.is_some());
    assert!(contact.range_m.is_some());
    assert_eq!(contact.range_m.unwrap() % 100.0, 0.0);
    assert_eq!(contact.bearing_deg.unwrap() % 5.0, 0.0);
    assert!(b.observe(Side::Red).contacts.is_empty());
}

#[test]
fn radar_contact_refreshes_and_does_not_reveal_the_ground() {
    let mut b = battle();
    let first = b.observe(Side::Blue).contacts[0].clone();
    for _ in 0..3 {
        b.step();
    }
    let refreshed = b.observe(Side::Blue).contacts[0].clone();
    assert_eq!(first.id, refreshed.id);
    assert_eq!(refreshed.source, ContactSource::Radar);
    assert!(!b
        .observe(Side::Blue)
        .ground_visibility
        .visible(500.0, 200.0));
}
