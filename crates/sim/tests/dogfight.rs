use contract::ids::{Side, UnitId};
use contract::observation::ObservedTargetId;
use serde_json::json;
use sim::air::dogfight::{breaks, intent};
use sim::air::{FixedWingState, Intent};
use sim::knowledge::Track;
use sim::math::{v2, v3};

fn track(position: [f64; 3], velocity: [f64; 2], yaw: f64) -> Track {
    Track {
        id: ObservedTargetId(1),
        last_seen: 0,
        position: v3(position[0], position[1], position[2]),
        yaw,
        velocity: v2(velocity[0], velocity[1]),
        members: Vec::new(),
        layer: contract::catalog::AltitudeLayer::HighAir,
        class: contract::weapons::TargetClass::AirFixedWing,
    }
}

fn own(position: [f64; 3], heading: f64) -> FixedWingState {
    FixedWingState {
        pos: v3(position[0], position[1], position[2]),
        heading,
        speed_mps: 200.0,
        burner: 0.0,
        turn_sign: 1,
    }
}

fn air() -> contract::scenario::FixedWingFlight {
    serde_json::from_value(sim::fixtures::test_rules()["air"]["fixed_wing"].clone()).unwrap()
}

/// Order `unit` of `side` to fly to `goal`: how a test points one jet at
/// another, so that its weapons bear for a reason rather than as its circle
/// happens to swing past.
fn fly_to(battle: &mut sim::battle::Battle, side: Side, seq: u64, unit: u32, goal: [f64; 2]) {
    let ack = battle.accept(contract::command::CommandEnvelope {
        side,
        player: None,
        seq,
        queued: false,
        order: contract::command::Order::Move {
            units: vec![UnitId(unit)],
            gesture: seq,
            goal,
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    });
    assert_eq!(ack.error, None);
}

fn air_battle(units: serde_json::Value) -> sim::battle::Battle {
    air_battle_with(sim::fixtures::test_rules(), units)
}

fn air_battle_with(mut rules: serde_json::Value, units: serde_json::Value) -> sim::battle::Battle {
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({"mounts": [{
            "id": "air missiles", "name": "Air Missiles", "weapons": ["air_missile"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }], "body": {"hull": {"hp": 10000}}}),
    );
    let setup = serde_json::from_value(json!({
        "map": {"size": [2400, 1000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35},
        "rules": rules, "units": units, "events": [], "scripts": []
    }))
    .unwrap();
    sim::battle::Battle::new(&setup, 1)
}

#[test]
fn firing_makes_it_your_target_and_it_adopts_you_only_if_free() {
    let mut battle = air_battle(json!([
        {"side":"blue", "kind":"test_jet", "position":[500,500], "yaw":0.0, "engagement":"fire_at_will"},
        {"side":"red", "kind":"test_jet", "position":[1300,500], "yaw":std::f64::consts::PI, "engagement":"return_fire_only"}
    ]));
    // The two fly at each other, head-on.
    fly_to(&mut battle, Side::Blue, 1, 0, [2300.0, 500.0]);
    fly_to(&mut battle, Side::Red, 1, 1, [100.0, 500.0]);
    for _ in 0..120 {
        battle.step();
    }
    assert_eq!(
        battle.unit(UnitId(0)).unwrap().air.unwrap().dogfight_target,
        Some(UnitId(1))
    );
    assert_eq!(
        battle.unit(UnitId(1)).unwrap().air.unwrap().dogfight_target,
        Some(UnitId(0))
    );
}

#[test]
fn a_third_jet_joins_by_targeting_an_engaged_one() {
    let mut battle = air_battle(json!([
        {"side":"blue", "kind":"test_jet", "position":[500,500], "yaw":0.0, "engagement":"fire_at_will"},
        {"side":"red", "kind":"test_jet", "position":[1300,500], "yaw":std::f64::consts::PI, "engagement":"return_fire_only"},
        {"side":"red", "kind":"test_jet", "position":[1500,500], "yaw":std::f64::consts::PI, "engagement":"fire_at_will"}
    ]));
    // Blue and the first red jet fly at each other, head-on, and the
    // second red jet is ordered onto blue once blue is engaged.
    fly_to(&mut battle, Side::Blue, 1, 0, [2300.0, 500.0]);
    fly_to(&mut battle, Side::Red, 1, 1, [100.0, 500.0]);
    for _ in 0..120 {
        battle.step();
        if battle
            .unit(UnitId(0))
            .unwrap()
            .air
            .unwrap()
            .dogfight_target
            .is_some()
        {
            break;
        }
    }
    assert_eq!(
        battle.unit(UnitId(0)).unwrap().air.unwrap().dogfight_target,
        Some(UnitId(1)),
        "blue engages the first red jet"
    );
    let blue = battle
        .observe(Side::Red)
        .identified
        .iter()
        .find(|t| t.position[0] < 1_000.0)
        .map(|t| t.id)
        .expect("red sees blue");
    let ack = battle.accept(contract::command::CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 2,
        queued: false,
        order: contract::command::Order::Attack {
            units: vec![UnitId(2)],
            target: contract::command::TargetRef::Identified { id: blue },
        },
    });
    assert_eq!(ack.error, None);
    // A second, well inside the break range of blue flying on.
    for _ in 0..30 {
        battle.step();
    }
    assert_eq!(
        battle.unit(UnitId(2)).unwrap().air.unwrap().dogfight_target,
        Some(UnitId(0))
    );
}

/// A jet ordered onto an enemy jet beyond the break range flies at it, and
/// the dogfight forms once it is in range: the order is not lost to the
/// break rule while the two are still far apart.
#[test]
fn a_jet_ordered_onto_a_far_jet_closes_until_the_dogfight_forms() {
    const BREAK_M: f64 = 1_000.0;
    let mut rules = sim::fixtures::test_rules();
    rules["air"]["fixed_wing"]["dogfight_break_m"] = json!(BREAK_M);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({"sensors": {"high_air_m": BREAK_M * 3.0}, "body": {"hull": {"hp": 10000}}}),
    );
    let setup = serde_json::from_value(json!({
        "map": {"size": [4000, 1500], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35},
        "rules": rules,
        "units": [
            {"side":"blue", "kind":"test_jet", "position":[600, 750], "yaw":std::f64::consts::FRAC_PI_2},
            {"side":"red", "kind":"test_jet", "position":[600.0 + BREAK_M * 2.0, 750], "yaw":std::f64::consts::FRAC_PI_2, "engagement":"return_fire_only"}
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    battle.step();
    let enemy = battle
        .observe(Side::Blue)
        .identified
        .first()
        .map(|t| t.id)
        .expect("blue sees the red jet");
    let ack = battle.accept(contract::command::CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: contract::command::Order::Attack {
            units: vec![UnitId(0)],
            target: contract::command::TargetRef::Identified { id: enemy },
        },
    });
    assert_eq!(ack.error, None);
    for _ in 0..30 * 30 {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().air.unwrap().dogfight_target == Some(UnitId(1)) {
            let apart = (battle.unit(UnitId(0)).unwrap().position
                - battle.unit(UnitId(1)).unwrap().position)
                .length();
            if apart <= BREAK_M {
                return;
            }
        }
    }
    panic!("the ordered jet never closed into a dogfight");
}

#[test]
fn the_link_breaks_past_the_rule_distance_in_3d() {
    let rules = air();
    assert!(!breaks(rules.dogfight_break_m, &rules));
    assert!(breaks(
        (rules.dogfight_break_m.powi(2) + 1.0).sqrt(),
        &rules
    ));
}

/// A dogfight forms only within one layer (D29c): a jet attacks a
/// helicopter below it in passes, and a helicopter firing up at the jet
/// hovers and shoots; neither links.
#[test]
fn a_jet_makes_passes_at_a_helicopter_and_neither_dogfights() {
    let mut rules = sim::fixtures::test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({"mounts": [{
            "id": "air missiles", "name": "Air Missiles", "weapons": ["air_missile"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }], "body": {"hull": {"hp": 10000}}}),
    );
    let mut battle = air_battle_with(
        rules,
        json!([
            {"side":"blue", "kind":"test_jet", "position":[500,500], "yaw":0.0, "engagement":"fire_at_will"},
            {"side":"red", "kind":"test_heli", "position":[1400,500], "yaw":std::f64::consts::PI, "engagement":"fire_at_will"}
        ]),
    );
    let (jet, heli) = (UnitId(0), UnitId(1));
    // The jet flies in to circle within the helicopter's reach.
    fly_to(&mut battle, Side::Blue, 1, 0, [1000.0, 500.0]);
    let mut passes = false;
    for _ in 0..600 {
        battle.step();
        let air = |id| battle.unit(id).unwrap().air.unwrap();
        assert_eq!(air(jet).dogfight_target, None);
        assert_eq!(air(heli).dogfight_target, None);
        passes |= air(jet).pass.is_some();
    }
    assert!(passes, "the jet attacks the helicopter in passes");
    assert!(
        battle.unit(jet).unwrap().attackers.contains(&heli),
        "the helicopter fired on the jet"
    );
}

/// Height never makes a dogfight: a helicopter hovering over a mesa,
/// higher above the map than half the jet's altitude, is still a low-air
/// rotor. The jet it fires on answers it in passes rather than circling
/// where it was, waiting for a dogfight that can never form.
#[test]
fn a_jet_answers_a_helicopter_above_it_on_high_ground_in_passes() {
    const MESA_M: f64 = 250.0;
    let mut rules = sim::fixtures::test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        json!({"mounts": [{
            "id": "air missiles", "name": "Air Missiles", "weapons": ["air_missile"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }], "body": {"hull": {"hp": 10000}}}),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({"mounts": [{
            "id": "gun", "name": "Forward Cannon", "weapons": ["air_gun"],
            "turret": false, "pivot_m": [0, 0, 1.8], "muzzle_m": [7, 0, 0]
        }], "body": {"hull": {"hp": 10000}}}),
    );
    let setup = serde_json::from_value(json!({
        "map": {
            "size": [2400, 1000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "relief": [{ "kind": "mesa", "rect": [1100, 250, 1700, 750], "height_m": MESA_M, "side_degrees": 40 }]
        },
        "rules": rules,
        "units": [
            {"side":"blue", "kind":"test_jet", "position":[500,500], "yaw":0.0, "engagement":"fire_at_will"},
            {"side":"red", "kind":"test_heli", "position":[1400,500], "yaw":std::f64::consts::PI, "engagement":"fire_at_will"},
            {"side":"blue", "kind":"test_recon", "position":[1250,500], "engagement":"return_fire_only"}
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    let (jet, heli) = (UnitId(0), UnitId(1));
    // The jet flies in to circle within the helicopter's reach.
    fly_to(&mut battle, Side::Blue, 1, 0, [1000.0, 500.0]);
    let mut fired_on = false;
    let mut passes = false;
    for _ in 0..600 {
        battle.step();
        let unit = |id| battle.unit(id).unwrap();
        if !fired_on && unit(jet).attackers.contains(&heli) {
            fired_on = true;
            assert!(
                unit(heli).position.z > unit(jet).position.z * 0.5,
                "the helicopter fires from above half the jet's height"
            );
            assert!(
                battle
                    .observe(Side::Blue)
                    .identified
                    .iter()
                    .any(|u| u.kind == unit(heli).kind),
                "the recon on the mesa sees the helicopter"
            );
        }
        assert_eq!(unit(jet).air.unwrap().dogfight_target, None);
        passes |= unit(jet).air.unwrap().pass.is_some();
    }
    assert!(fired_on, "the helicopter fired on the jet");
    assert!(passes, "the jet attacks the helicopter in passes");
}

#[test]
fn on_a_head_on_merge_both_break_away() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let target = track([100.0, 0.0, 250.0], [-200.0, 0.0], std::f64::consts::PI);
    let Intent::Pursue { point, .. } = intent(&own, &target, 7, &air()) else {
        panic!("pursuit")
    };
    assert!(
        point.x < own.pos.x,
        "merge point should be behind own jet: {point:?}"
    );
}

#[test]
fn pursuit_aims_where_the_target_was() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let target = track([100.0, 0.0, 250.0], [50.0, 0.0], 0.0);
    let Intent::Pursue {
        point, vel, lag_s, ..
    } = intent(&own, &target, 7, &air())
    else {
        panic!("pursuit")
    };
    assert_eq!(point.x, target.position.x);
    assert_eq!(vel.x, target.velocity.x);
    assert!(lag_s > 0.0);
}

#[test]
fn dogfight_height_stays_in_its_band() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let target = track([100.0, 0.0, 250.0], [0.0, 0.0], 0.0);
    let Intent::Pursue { agl, .. } = intent(&own, &target, 7, &air()) else {
        panic!("pursuit")
    };
    assert!((air().dogfight_band_agl_m[0]..=air().dogfight_band_agl_m[1]).contains(&agl));
}
