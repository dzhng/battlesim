//! Radar signatures are a separate, SEAD-only observation channel.

use std::collections::BTreeSet;

use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
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
            "mobility": { "air": { "layer": "high_air", "flight": "fixed_wing" } },
            "capabilities": { "air": {
                "fuel_capacity_s": 180.0,
                "sead": true,
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

#[test]
fn anti_radiation_weapon_can_fire_at_radar_contact_without_unit_los() {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air", "flight": "fixed_wing" } },
            "capabilities": { "air": {
                "fuel_capacity_s": 180.0,
                "sead": true,
                "radar": { "emits": false, "detection_m": 1200.0 },
                "refund": { "health": 0.34, "ammunition": 0.33, "fuel": 0.33 }
            }},
            "mounts": [{ "id": "HMG", "weapons": ["sead_missile"] }]
        }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({
            "capabilities": {
                "radar": { "emits": true, "detection_m": 1200.0 }
            }
        }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_heli", "position": [100, 200] },
            { "side": "red", "kind": "test_jeep", "position": [500, 200] }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    for _ in 0..120 {
        battle.step();
        if !battle.observe(Side::Blue).guided.is_empty() {
            return;
        }
    }
    panic!("anti-radiation weapon never launched at its radar contact");
}

#[test]
fn ground_mission_can_strafe_while_bombs_remain() {
    use sim::air::pass::{choose_weapon, PassWeapon};
    assert_eq!(
        choose_weapon(&[PassWeapon::Gun, PassWeapon::Bomb]),
        Some(PassWeapon::Bomb)
    );
    assert_eq!(choose_weapon(&[PassWeapon::Gun]), Some(PassWeapon::Gun));
}

#[test]
fn high_air_ground_attack_uses_bombs_then_strafing_after_the_store_is_empty() {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air", "flight": "fixed_wing" } },
            "sensors": { "ground_m": 1200.0 },
            "capabilities": { "air": {
                "fuel_capacity_s": 180.0,
                "afterburner": {},
                "refund": { "health": 0.34, "ammunition": 0.33, "fuel": 0.33 }
            }},
            "mounts": [
                { "id": "HMG", "weapons": ["air_gun"], "turret": true,
                  "pivot_m": [1.6, 1.12, 1.25], "muzzle_m": [1.2, 0, 0] },
                { "id": "bombs", "name": "Bombs", "weapons": ["air_bomb"], "turret": false,
                  "pivot_m": [0, 0, 0], "muzzle_m": [1.2, 0, 0] }
            ]
        }),
    );
    // One bomb is enough to prove the transition without making this test
    // wait through the authored ten-second aircraft reload interval.
    rules["weapons"]["air_bomb"]["ammo"] = json!(1);
    rules["weapons"]["air_bomb"]["aim_s"] = json!(0.1);
    let setup = serde_json::from_value(json!({
        "map": { "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [{ "side": "blue", "kind": "test_heli", "position": [200, 300] }],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        slot: None,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [500.0, 300.0, 0.0],
            },
        },
        queued: false,
    });
    assert_eq!(ack.error, None);

    let mut seen = BTreeSet::new();
    let mut launches = Vec::new();
    for _ in 0..900 {
        battle.step();
        for (projectile, round) in battle.rounds() {
            if seen.insert(projectile.id) && round.unit == UnitId(0) {
                launches.push(battle.arsenal().weapons[round.weapon].id.clone());
            }
        }
        if launches.iter().any(|id| id == "air_bomb") && launches.iter().any(|id| id == "air_gun") {
            break;
        }
    }
    assert_eq!(
        launches.first().map(String::as_str),
        Some("air_bomb"),
        "{launches:?}"
    );
    assert!(
        launches.iter().any(|id| id == "air_gun"),
        "bomb store never fell back to strafing: {launches:?}"
    );
}

#[test]
fn sead_missile_has_motor_lifetime_to_reach_its_authored_long_range() {
    let row = &sim::fixtures::test_game()["weapons"]["sead_missile"];
    let range = row["range_m"].as_f64().unwrap();
    let launch_speed = row["speed_mps"].as_f64().unwrap();
    let top_speed = row["top_speed_mps"].as_f64().unwrap();
    let acceleration = row["accel_mps2"].as_f64().unwrap();
    let lifetime = row["lifetime_s"].as_f64().unwrap();
    let motor_time = (top_speed - launch_speed) / acceleration;
    let motor_distance = launch_speed * motor_time
        + 0.5 * acceleration * motor_time * motor_time
        + top_speed * (lifetime - motor_time).max(0.0);
    assert!(range >= 6500.0, "SEAD range regressed: {range} m");
    assert!(
        motor_distance >= range,
        "SEAD expires after {motor_distance:.0} m, short of {range:.0} m"
    );
}

#[test]
fn long_sam_motor_lifetime_reaches_its_authored_range() {
    let row = &sim::fixtures::test_game()["weapons"]["aa_long"];
    let range = row["range_m"].as_f64().unwrap();
    let launch_speed = row["speed_mps"].as_f64().unwrap();
    let top_speed = row["top_speed_mps"].as_f64().unwrap();
    let acceleration = row["accel_mps2"].as_f64().unwrap();
    let lifetime = row["lifetime_s"].as_f64().unwrap();
    let motor_time = (top_speed - launch_speed) / acceleration;
    let distance = launch_speed * motor_time
        + 0.5 * acceleration * motor_time * motor_time
        + top_speed * (lifetime - motor_time).max(0.0);
    assert!(
        distance >= range,
        "long SAM expires after {distance:.0} m, short of {range:.0} m"
    );
}

/// An emitter on any side of the SEAD aircraft reaches the browser with its
/// bearing and range: the bearing is a compass-style 0–360° value, so the
/// publication's "absent" never swallows one to the south.
#[test]
fn every_radar_contact_reaches_the_reader_with_its_bearing_and_range() {
    use sim::publication::{self, reader::Reader};
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air", "flight": "fixed_wing" } },
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
        json!({ "capabilities": {
            "radar": { "emits": true, "detection_m": 1200.0 }
        }}),
    );
    // Emitters north-east, south-east, south-west and north-west of it.
    let setup = serde_json::from_value(json!({
        "map": { "size": [800, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_heli", "position": [400, 400] },
            { "side": "red", "kind": "test_jeep", "position": [700, 600] },
            { "side": "red", "kind": "test_jeep", "position": [700, 150] },
            { "side": "red", "kind": "test_jeep", "position": [100, 150] },
            { "side": "red", "kind": "test_jeep", "position": [100, 650] }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    battle.step();
    let layout = publication::layout_json(&battle);
    let mut reader = Reader::new(&layout).unwrap();
    let words: Vec<u32> = publication::Publisher::new()
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|w| w.to_bits())
        .collect();
    let o = reader.decode(&words).unwrap().unwrap();
    let radar: Vec<_> = o
        .contacts
        .iter()
        .filter(|c| &*c.source == "radar")
        .collect();
    assert_eq!(radar.len(), 4, "{:?}", o.contacts);
    for c in radar {
        let bearing = c
            .bearing_deg
            .expect("every radar contact keeps its bearing");
        assert!(
            (0.0..360.0).contains(&bearing) && bearing % 5.0 == 0.0,
            "{bearing}"
        );
        assert!(c.range_m.is_some_and(|r| r > 0.0 && r % 100.0 == 0.0));
    }
}

/// The test rules with `test_heli` flying as a SEAD jet whose radar reaches
/// `reach_m`, and `test_jeep` carrying a radar that emits or not.
fn sead_rules(reach_m: f64, jeep_emits: bool) -> serde_json::Value {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air", "flight": "fixed_wing" } },
            "capabilities": { "air": {
                "fuel_capacity_s": 180.0,
                "sead": true,
                "radar": { "emits": false, "detection_m": reach_m },
                "refund": { "health": 0.34, "ammunition": 0.33, "fuel": 0.33 }
            }}
        }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "capabilities": {
            "radar": { "emits": jeep_emits, "detection_m": if jeep_emits { reach_m } else { 0.0 } }
        }}),
    );
    rules
}

#[test]
fn a_sead_jet_finds_an_emitting_radar_but_not_a_silent_one() {
    let reach = 1000.0;
    let radar_contacts = |emits: bool| {
        let setup = serde_json::from_value(json!({
            "map": { "size": [2.0 * reach, reach], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": sead_rules(reach, emits),
            "units": [
                { "side": "red", "kind": "test_heli", "position": [0.5 * reach, 0.5 * reach], "yaw": 0.0 },
                { "side": "blue", "kind": "test_jeep", "position": [reach, 0.5 * reach] }
            ],
            "events": [], "scripts": []
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 1);
        b.step();
        b.observe(Side::Red)
            .contacts
            .iter()
            .filter(|c| c.source == ContactSource::Radar)
            .count()
    };
    assert_eq!(radar_contacts(true), 1, "an emitting radar is found");
    assert_eq!(radar_contacts(false), 0, "a silent one is not");
}

#[test]
fn an_emitting_radar_sees_high_air_to_its_detection_range() {
    // Emitting is the cost of a radar; its reach is the payoff: a jet half
    // its reach out is seen, one past it is not.
    let reach = 1000.0;
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "capabilities": {
            "radar": { "emits": true, "detection_m": reach }
        }}),
    );
    let seen = |across_m: f64| {
        let setup = serde_json::from_value(json!({
            "map": { "size": [3.0 * reach, reach], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": rules.clone(),
            "units": [
                { "side": "blue", "kind": "test_jeep", "position": [0.5 * reach, 0.5 * reach], "yaw": 0.0 },
                { "side": "red", "kind": "test_jet", "position": [0.5 * reach + across_m, 0.5 * reach],
                  "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" }
            ],
            "events": [], "scripts": []
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 1);
        b.step();
        b.step();
        !b.observe(Side::Blue).identified.is_empty()
    };
    assert!(
        seen(0.5 * reach),
        "a jet half the radar's reach out is seen"
    );
    assert!(!seen(1.5 * reach), "one past its reach is not");
}

/// Whether `spotter` of the test rules, with `unaided` as the rules' high-air
/// sight without radar and `patch` applied to it, identifies the test jet
/// cruising in high air `across_m` away across the ground. The spotter faces
/// the jet, which flies abeam, so it stays about that far off.
fn spots_jet(
    spotter: &str,
    patch: serde_json::Value,
    unaided: serde_json::Value,
    across_m: f64,
) -> bool {
    let mut rules = sim::fixtures::test_game();
    rules["sensors"]["unaided_high_air_m"] = unaided;
    sim::fixtures::patch_catalog(&mut rules, "units", spotter, patch);
    let size = 2.0 * across_m + 400.0;
    let setup = serde_json::from_value(json!({
        "map": { "size": [size, 400.0], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": spotter, "position": [200, 200], "yaw": 0.0,
              "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_jet", "position": [200.0 + across_m, 200.0],
              "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    b.step();
    b.step();
    !b.observe(Side::Blue).identified.is_empty()
}

#[test]
fn without_radar_a_unit_sees_a_cruising_jet_as_far_as_its_body_looks_up() {
    // Soldiers on foot, an aircraft's crew and a ground vehicle's crew each
    // see high air as far as the rules' unaided row for them says (slant
    // range to a jet at cruise height): within half that reach across the
    // ground, never past one and a half times it. A unit authoring 0 is
    // blind even directly beneath the jet.
    let unaided = json!({ "on_foot": 900.0, "aircraft": 800.0, "ground_vehicle": 700.0 });
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    let cruise = rules.air.high_air.cruise_agl_m;
    for (spotter, reach) in [
        ("test_rifle", 900.0),
        ("test_heli", 800.0),
        ("test_supply", 700.0),
    ] {
        assert!(
            (0.25 * reach * reach + cruise * cruise).sqrt() < reach,
            "half of {reach} m across is inside the slant reach to {cruise} m up"
        );
        assert!(
            spots_jet(spotter, json!({}), unaided.clone(), 0.5 * reach),
            "{spotter} sees it {} m off",
            0.5 * reach
        );
        assert!(
            !spots_jet(spotter, json!({}), unaided.clone(), 1.5 * reach),
            "{spotter} does not see it {} m off",
            1.5 * reach
        );
    }
    let blind = json!({ "sensors": { "high_air_m": 0.0 } });
    assert!(
        !spots_jet("test_tank", blind, unaided, 0.0),
        "a unit authoring 0 never sees it, even directly beneath"
    );
}
