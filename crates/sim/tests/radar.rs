//! Radar signatures are a separate, SEAD-only observation channel.

use std::collections::BTreeSet;

use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::ContactSource;
use contract::weapons::GroundAttackKind;
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

#[test]
fn anti_radiation_weapon_can_fire_at_radar_contact_without_unit_los() {
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
                "air_defense": { "tier": "long" },
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
fn ground_mission_prefers_available_bombs_then_falls_back_to_strafe() {
    let rules = sim::fixtures::test_game();
    let gun = &rules["weapons"]["hmg"];
    let mut bomb_value = gun.clone();
    bomb_value["ground_attack"] = json!("bomb");
    bomb_value["ammo"] = json!(4);
    let bomb: contract::weapons::WeaponDefinition = serde_json::from_value(bomb_value).unwrap();
    let mut gun_value = gun.clone();
    gun_value["ground_attack"] = json!("strafe");
    let gun: contract::weapons::WeaponDefinition = serde_json::from_value(gun_value).unwrap();
    let rows = sim::weapons::ground_attack_kind([(&bomb, Some(4)), (&gun, None)]);
    assert_eq!(rows, Some(GroundAttackKind::Bomb));
    let fallback = sim::weapons::ground_attack_kind([(&bomb, Some(0)), (&gun, None)]);
    assert_eq!(fallback, Some(GroundAttackKind::Strafe));
}

#[test]
fn high_air_ground_attack_uses_bombs_then_strafing_after_the_store_is_empty() {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air" } },
            "sensors": { "ground_m": 1200.0 },
            "capabilities": { "air": {
                "fuel_capacity_s": 180.0,
                "afterburner": {},
                "refund": { "health": 0.34, "ammunition": 0.33, "fuel": 0.33 }
            }},
            "mounts": [
                { "id": "HMG", "weapons": ["air_gun"], "turret": true,
                  "pivot_m": [1.6, 1.12, 1.25], "muzzle_m": [1.2, 0, 0] },
                { "id": "bombs", "name": "Bombs", "weapons": ["air_bomb_guided"], "turret": false,
                  "pivot_m": [0, 0, 0], "muzzle_m": [1.2, 0, 0] }
            ]
        }),
    );
    // One bomb is enough to prove the transition without making this test
    // wait through the authored ten-second aircraft reload interval.
    rules["weapons"]["air_bomb_guided"]["ammo"] = json!(1);
    rules["weapons"]["air_bomb_guided"]["aim_s"] = json!(0.1);
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
        if launches.iter().any(|id| id == "air_bomb_guided")
            && launches.iter().any(|id| id == "air_gun")
        {
            break;
        }
    }
    assert_eq!(
        launches.first().map(String::as_str),
        Some("air_bomb_guided"),
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
        json!({ "capabilities": {
            "air_defense": { "tier": "long" },
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
