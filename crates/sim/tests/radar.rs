//! Radar signatures are a separate observation channel: a unit with a radar
//! receiver finds the enemy radars that emit, by bearing and range, and only
//! an anti-radiation weapon fires at what it finds.

use std::collections::BTreeSet;

use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::ContactSource;
use serde_json::json;
use sim::battle::Battle;

/// The test rules with `test_supply` carrying a radar receiver that reaches
/// `receiver_m`, `test_tank` a radar that emits, and `test_jeep` armed with
/// `jeep_weapon`.
fn receiver_rules(receiver_m: f64, jeep_weapon: &str) -> serde_json::Value {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        json!({ "capabilities": { "radar_receiver": { "range_m": receiver_m } } }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({ "capabilities": { "radar_emitter": true } }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "mounts": [{ "id": "HMG", "weapons": [jeep_weapon] }] }),
    );
    rules
}

fn battle_of(rules: &serde_json::Value, size: [f64; 2], units: serde_json::Value) -> Battle {
    let setup = serde_json::from_value(json!({
        "map": { "size": size, "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": units,
        "events": [], "scripts": []
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

fn radar_contacts(b: &Battle, side: Side) -> Vec<contract::observation::ApproximateContact> {
    b.observe(side)
        .contacts
        .iter()
        .filter(|c| c.source == ContactSource::Radar)
        .cloned()
        .collect()
}

/// Shots fired by `unit`'s mounts so far.
fn shots(b: &Battle, unit: UnitId) -> u32 {
    b.unit(unit).unwrap().mounts.iter().map(|m| m.shots).sum()
}

#[test]
fn a_ground_unit_with_a_radar_receiver_finds_an_emitter_within_its_reach() {
    // A truck with a receiver reaching 1,000 m: an emitting radar 700 m off,
    // past what its crew sees, is found by bearing and range; one 1,100 m
    // off is not.
    let rules = receiver_rules(1000.0, "hmg");
    let found = |across: f64| {
        let b = battle_of(
            &rules,
            [1400.0, 400.0],
            json!([
                { "side": "blue", "kind": "test_supply", "position": [100, 200] },
                { "side": "red", "kind": "test_tank", "position": [100.0 + across, 200.0] }
            ]),
        );
        radar_contacts(&b, Side::Blue)
    };
    let near = found(700.0);
    let [contact] = near.as_slice() else {
        panic!("expected one radar contact: {near:?}");
    };
    assert!(contact.kind.is_none(), "radar never identifies its emitter");
    assert_eq!(contact.bearing_deg, Some(0.0));
    assert_eq!(contact.range_m, Some(700.0));
    assert!(found(1100.0).is_empty(), "past its reach it finds nothing");
}

#[test]
fn a_radar_report_rounds_by_the_rules_steps() {
    // Steps of 45° and 500 m, an area at least 300 m wide: an emitter 707 m
    // off, 8° left of east, reads as bearing 0° and range 500 m.
    let mut rules = receiver_rules(1000.0, "hmg");
    rules["sensors"]["radar_report"] =
        json!({ "bearing_step_deg": 45, "range_step_m": 500, "min_radius_m": 300 });
    let b = battle_of(
        &rules,
        [1400.0, 400.0],
        json!([
            { "side": "blue", "kind": "test_supply", "position": [100, 100] },
            { "side": "red", "kind": "test_tank", "position": [800, 200] }
        ]),
    );
    let contacts = radar_contacts(&b, Side::Blue);
    let [contact] = contacts.as_slice() else {
        panic!("expected one radar contact: {contacts:?}");
    };
    assert_eq!(contact.bearing_deg, Some(0.0));
    assert_eq!(contact.range_m, Some(500.0));
    assert!(contact.radius >= 300.0, "{}", contact.radius);
}

#[test]
fn a_receiver_finds_an_airborne_emitter_and_a_ground_only_anti_radiation_row_holds() {
    // A helicopter whose radar emits, hovering 700 m off past what the
    // jeep sees, is found like any emitter, at its own layer; the jeep's
    // anti-radiation missile reaches only the ground, so it never fires.
    let mut rules = receiver_rules(3000.0, "sead_missile");
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({ "capabilities": { "radar_emitter": true } }),
    );
    let mut b = battle_of(
        &rules,
        [1400.0, 400.0],
        json!([
            { "side": "blue", "kind": "test_jeep", "position": [100, 200], "yaw": 0.0 },
            { "side": "blue", "kind": "test_supply", "position": [100, 260] },
            { "side": "red", "kind": "test_heli", "position": [800, 200], "engagement": "return_fire_only" }
        ]),
    );
    let contacts = radar_contacts(&b, Side::Blue);
    let [contact] = contacts.as_slice() else {
        panic!("expected one radar contact: {contacts:?}");
    };
    assert_eq!(contact.layer, contract::catalog::AltitudeLayer::LowAir);
    for _ in 0..300 {
        b.step();
    }
    assert_eq!(shots(&b, UnitId(0)), 0);
}

#[test]
fn radar_contact_refreshes_and_does_not_reveal_the_ground() {
    let rules = receiver_rules(1000.0, "hmg");
    let mut b = battle_of(
        &rules,
        [1400.0, 400.0],
        json!([
            { "side": "blue", "kind": "test_supply", "position": [100, 200] },
            { "side": "red", "kind": "test_tank", "position": [800, 200] }
        ]),
    );
    let first = radar_contacts(&b, Side::Blue)[0].clone();
    for _ in 0..3 {
        b.step();
    }
    let refreshed = radar_contacts(&b, Side::Blue)[0].clone();
    assert_eq!(first.id, refreshed.id);
    assert!(!b
        .observe(Side::Blue)
        .ground_visibility
        .visible(800.0, 200.0));
}

#[test]
fn an_anti_radiation_weapon_without_a_receiver_fires_at_a_radar_contact_a_receiver_found() {
    // A jeep with an anti-radiation launcher and no receiver, 2,000 m from
    // an emitting radar it cannot see: alone it never fires; with a truck
    // beside it whose receiver finds the radar, it fires at that contact.
    let rules = receiver_rules(3000.0, "sead_missile");
    let launches = |with_receiver: bool| {
        let mut units = vec![
            json!({ "side": "blue", "kind": "test_jeep", "position": [100, 200], "yaw": 0.0 }),
            json!({ "side": "red", "kind": "test_tank", "position": [2100, 200] }),
        ];
        if with_receiver {
            units.push(json!({ "side": "blue", "kind": "test_supply", "position": [100, 260] }));
        }
        let mut b = battle_of(&rules, [2400.0, 400.0], json!(units));
        for _ in 0..300 {
            b.step();
        }
        assert!(b.observe(Side::Blue).identified.is_empty());
        shots(&b, UnitId(0))
    };
    assert_eq!(
        launches(false),
        0,
        "without a receiver nothing finds the radar"
    );
    assert!(
        launches(true) > 0,
        "it fires at the contact a receiver found"
    );
}

/// A battle of `rules` in which a blue jeep at the west end faces a red tank
/// 700 m east, past what it sees. The tank's radar emits and a blue truck's
/// receiver finds it (`radar`), or it is silent and fires now and then, so
/// only its shots are heard (`firing`). Returns the jeep's shots and the
/// sources of blue's contacts.
fn at_an_unseen_tank(rules: &serde_json::Value, radar: bool) -> (u32, Vec<ContactSource>) {
    let mut rules = rules.clone();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({ "capabilities": { "radar_emitter": radar } }),
    );
    let mut units = vec![
        json!({ "side": "blue", "kind": "test_jeep", "position": [100, 200], "yaw": 0.0 }),
        json!({ "side": "red", "kind": "test_tank", "position": [800, 200], "engagement": "return_fire_only" }),
    ];
    if radar {
        units.push(json!({ "side": "blue", "kind": "test_supply", "position": [100, 260] }));
    }
    let events: Vec<_> = if radar {
        Vec::new()
    } else {
        (0..10)
            .map(|k| json!({ "tick": 5 + 30 * k, "fire": { "unit": 1 } }))
            .collect()
    };
    let setup = serde_json::from_value(json!({
        "map": { "size": [1400, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": units,
        "events": events, "scripts": []
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let mut sources = Vec::new();
    for _ in 0..300 {
        b.step();
        for c in &b.observe(Side::Blue).contacts {
            if !sources.contains(&c.source) {
                sources.push(c.source);
            }
        }
    }
    assert!(b.observe(Side::Blue).identified.is_empty());
    (shots(&b, UnitId(0)), sources)
}

#[test]
fn a_general_purpose_gun_area_fires_at_a_radar_contact_as_at_a_heard_one() {
    // A radar contact is an area like any other: with no enemy it can see,
    // the jeep's high-explosive gun shells where the radar was found, as it
    // shells where shots were heard.
    let rules = receiver_rules(3000.0, "tank_he");
    let (heard, sources) = at_an_unseen_tank(&rules, false);
    assert!(!sources.contains(&ContactSource::Radar), "{sources:?}");
    assert!(heard > 0, "it fires at the heard contact: {sources:?}");
    let (found, sources) = at_an_unseen_tank(&rules, true);
    assert!(sources.contains(&ContactSource::Radar), "{sources:?}");
    assert!(found > 0, "it fires at the radar contact");
}

#[test]
fn an_anti_radiation_row_fires_at_a_radar_contact_but_not_at_a_heard_one() {
    // It homes on emissions: where shots were heard there is nothing to home
    // on, even for a row that fires at men on the ground.
    let mut rules = receiver_rules(3000.0, "sead_missile");
    rules["weapons"]["sead_missile"]["targets"]["ground"]["classes"] =
        json!(["infantry", "vehicle"]);
    let (heard, sources) = at_an_unseen_tank(&rules, false);
    assert!(sources.contains(&ContactSource::Firing), "{sources:?}");
    assert_eq!(heard, 0, "it never fires at the heard contact");
    let (found, _) = at_an_unseen_tank(&rules, true);
    assert!(found > 0, "it fires at the radar contact");
}

#[test]
fn a_tank_fires_at_an_identified_emitting_radar() {
    // A radar that emits is found from afar; once seen, it is a vehicle like
    // any other. A tank 250 m from a jeep carrying one shoots at it.
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "capabilities": { "radar_emitter": true } }),
    );
    let mut b = battle_of(
        &rules,
        [800.0, 400.0],
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 200], "yaw": 0.0 },
            { "side": "red", "kind": "test_jeep", "position": [350, 200], "engagement": "return_fire_only" }
        ]),
    );
    for _ in 0..200 {
        b.step();
    }
    assert!(
        shots(&b, UnitId(0)) > 0,
        "the tank never fired at the radar it sees"
    );
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
                "afterburner": { "speed_factor": 1.5, "fuel_factor": 2.0 },
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
        player: None,
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
    let range = crate::common::weapon("sead_missile").range_m;
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
    let range = crate::common::weapon("aa_long").range_m;
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

/// An emitter on any side of a radar receiver reaches the browser with its
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
            "capabilities": { "radar_receiver": { "range_m": 1200.0 } }
        }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "capabilities": {
            "radar_emitter": true
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

/// The test rules with `test_heli` flying as a SEAD jet whose radar receiver
/// reaches `reach_m`, and `test_jeep` carrying a radar that emits or not.
fn sead_rules(reach_m: f64, jeep_emits: bool) -> serde_json::Value {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air", "flight": "fixed_wing" } },
            "capabilities": { "radar_receiver": { "range_m": reach_m } }
        }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "capabilities": {
            "radar_emitter": jeep_emits
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

/// Band reaches are independent, including on an emitter; zero air sight is blind.
#[test]
fn each_bands_sight_is_the_units_own_and_zero_is_blind() {
    let seen = |target: &str, band: &str, reach: f64, across: f64, emits: bool| {
        let mut rules = sim::fixtures::test_game();
        let mut sensors = json!({
            "ground_m": 200, "low_air_m": 600, "high_air_m": 1000,
            "sight_shape": { "front": 1, "side": 1, "rear": 1 }
        });
        sensors[band] = json!(reach);
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_jeep",
            json!({
                "sensors": sensors, "capabilities": { "radar_emitter": emits }
            }),
        );
        let setup = serde_json::from_value(json!({
            "map": { "size": [2000, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": rules,
            "units": [
                { "side": "blue", "kind": "test_jeep", "position": [100, 200], "engagement": "return_fire_only" },
                { "side": "red", "kind": target, "position": [100.0 + across, 200],
                  "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" }
            ],
            "events": [], "scripts": []
        })).unwrap();
        let mut battle = Battle::new(&setup, 1);
        battle.step();
        battle.step();
        !battle.observe(Side::Blue).identified.is_empty()
    };
    for emits in [false, true] {
        for (target, band, reach) in [
            ("test_jeep", "ground_m", 200.0),
            ("test_heli", "low_air_m", 600.0),
            ("test_jet", "high_air_m", 1000.0),
        ] {
            assert!(
                seen(target, band, reach, 0.5 * reach, emits),
                "inside {band}, emits={emits}"
            );
            assert!(
                !seen(target, band, reach, 1.5 * reach, emits),
                "outside {band}, emits={emits}"
            );
            if band != "ground_m" {
                assert!(
                    !seen(target, band, 0.0, 10.0, emits),
                    "blind to {band}, emits={emits}"
                );
            }
        }
    }
}
