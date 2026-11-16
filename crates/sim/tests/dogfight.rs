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
        turn_rate_rad_s: 0.0,
        turn_sign: 1,
    }
}

/// The passing distance the dogfight tests fly beside a target at.
const SEPARATION_M: f64 = 14.0;
/// The turn rate the dogfight tests' jets turn at by choice.
const TURN_RAD_S: f64 = 1.0;
/// The passes the flight-law tests shape: the field's middle far off, north.
const GEOMETRY: sim::air::dogfight::DogfightGeometry = sim::air::dogfight::DogfightGeometry {
    separation_m: SEPARATION_M,
    turn_rad_s: TURN_RAD_S,
    centre: sim::math::V2 { x: 0.0, y: 5000.0 },
    reach_m: 900.0,
    seen_behind_m: 3000.0,
    seen_abeam_m: 3000.0,
};

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

/// Attack-move `unit` to `goal`, where it circles hunting.
fn hunt_at(battle: &mut sim::battle::Battle, side: Side, seq: u64, unit: u32, goal: [f64; 2]) {
    let ack = battle.accept(contract::command::CommandEnvelope {
        side,
        player: None,
        seq,
        queued: false,
        order: contract::command::Order::AttackMove {
            units: vec![UnitId(unit)],
            gesture: seq,
            goal,
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

/// Exactly at the break range a pair is still inside it; a hair beyond,
/// past it.
#[test]
fn a_pair_is_past_the_break_only_beyond_its_range() {
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
    // The jet flies in to hunt within the helicopter's reach.
    hunt_at(&mut battle, Side::Blue, 1, 0, [1000.0, 500.0]);
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
/// rotor. The hunting jet it fires on answers it in passes rather than
/// circling where it was, waiting for a dogfight that can never form.
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
    // The jet flies in to hunt within the helicopter's reach.
    hunt_at(&mut battle, Side::Blue, 1, 0, [1000.0, 500.0]);
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

fn aim(own: &FixedWingState, target: &Track) -> sim::math::V2 {
    match intent(own, target, 7, &air(), &GEOMETRY, &mut None) {
        Intent::Pursue { point, .. } => point,
        other => panic!("pursuit, got {other:?}"),
    }
}

/// Head-on, a jet flies at the oncoming one nose-on, so its guns bear, and
/// from as far out as it needs to slide aside aims just to its right, so
/// the two pass beside each other.
#[test]
fn head_on_it_flies_at_the_target_nose_on_then_aims_beside_it() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let oncoming = |x: f64| track([x, 0.0, 250.0], [-200.0, 0.0], std::f64::consts::PI);
    let far = oncoming(3000.0);
    assert_eq!(aim(&own, &far), far.position.xy(), "nose-on far out");
    let near = oncoming(300.0);
    let beside = aim(&own, &near);
    assert_eq!(beside.x, near.position.x);
    assert!(
        beside.y < 0.0 && beside.y > -2.0 * SEPARATION_M,
        "beside it, to its right: {beside:?}"
    );
}

/// Past the other jet while both fly apart, it extends straight on until
/// they are as far apart as its weapons reach, so they all bear when it
/// comes back nose-on; then it turns back for the next pass.
#[test]
fn past_the_target_it_extends_to_its_reach_then_comes_round() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let behind = |d: f64| track([-d, 0.0, 250.0], [-200.0, 0.0], std::f64::consts::PI);
    let reach = GEOMETRY.reach_m;
    assert!(
        aim(&own, &behind(reach - 50.0)).x > own.pos.x,
        "extends straight on inside its reach"
    );
    assert!(
        aim(&own, &behind(reach + 50.0)).x < own.pos.x,
        "comes round toward it beyond its reach"
    );
}

/// A jet that sees only a short way behind it comes round before its
/// reach, so it keeps the other in sight as the pair spreads in the turn.
#[test]
fn a_short_sighted_jet_comes_round_inside_its_reach() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let behind = track([-400.0, 0.0, 250.0], [-200.0, 0.0], std::f64::consts::PI);
    let short_sighted = sim::air::dogfight::DogfightGeometry {
        seen_behind_m: 600.0,
        seen_abeam_m: 600.0,
        ..GEOMETRY
    };
    let point = |geometry| match intent(&own, &behind, 7, &air(), geometry, &mut None) {
        Intent::Pursue { point, .. } => point,
        other => panic!("pursuit, got {other:?}"),
    };
    assert!(point(&GEOMETRY).x > own.pos.x, "it would extend");
    assert!(
        point(&short_sighted).x < own.pos.x,
        "it comes round while it can still see the other"
    );
}

/// With the other jet on its tail, it comes round at once rather than
/// flying straight on while it is shot at.
#[test]
fn with_a_jet_on_its_tail_it_comes_round() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let on_tail = track([-150.0, 0.0, 250.0], [200.0, 0.0], 0.0);
    assert!(
        aim(&own, &on_tail).x < own.pos.x,
        "it turns toward the pursuer"
    );
}

/// A target that runs gets a nose-on chase from behind, never a sidestep.
#[test]
fn a_running_target_gets_a_nose_on_chase() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let running = track([100.0, 0.0, 250.0], [180.0, 0.0], 0.0);
    assert_eq!(aim(&own, &running), running.position.xy());
}

#[test]
fn dogfight_height_stays_in_its_band() {
    let own = own([0.0, 0.0, 250.0], 0.0);
    let target = track([100.0, 0.0, 250.0], [0.0, 0.0], 0.0);
    let Intent::Pursue { agl, .. } = intent(&own, &target, 7, &air(), &GEOMETRY, &mut None) else {
        panic!("pursuit")
    };
    assert!((air().dogfight_band_agl_m[0]..=air().dogfight_band_agl_m[1]).contains(&agl));
}

/// Two jets on the run field, blue at the left and red at the right, flying
/// at each other, each carrying `weapons` (and, given, a hull of `hp`), and
/// each ordered onto the other once it sees it.
fn duel(weapons: &[&str], hp: Option<u32>) -> sim::battle::Battle {
    duel_with(crate::attack_runs::run_rules(weapons), hp)
}

fn duel_with(mut rules: serde_json::Value, hp: Option<u32>) -> sim::battle::Battle {
    use crate::attack_runs::{run_field, RUN_START};
    let [x, y] = RUN_START;
    if let Some(hp) = hp {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_jet",
            json!({ "body": { "hull": { "hp": hp } } }),
        );
    }
    let mut battle = run_field(
        rules,
        json!([
            { "side": "blue", "kind": "test_jet", "position": [x, y], "yaw": 0.0, "engagement": "fire_at_will" },
            { "side": "red", "kind": "test_jet", "position": [x + 2000.0, y], "yaw": std::f64::consts::PI, "engagement": "fire_at_will" },
        ]),
    );
    // They fly at each other until blue sees red.
    fly_to(&mut battle, Side::Blue, 1, 0, [x + 3000.0, y]);
    fly_to(&mut battle, Side::Red, 1, 1, [x - 900.0, y]);
    // Once each sees the other, each is ordered onto it.
    for (side, unit) in [(Side::Blue, 0), (Side::Red, 1)] {
        let seen = (0..600).find_map(|_| {
            let id = battle.observe(side).identified.first().map(|t| t.id);
            if id.is_none() {
                battle.step();
            }
            id
        });
        let ack = battle.accept(contract::command::CommandEnvelope {
            side,
            player: None,
            seq: 2,
            queued: false,
            order: contract::command::Order::Attack {
                units: vec![UnitId(unit)],
                target: contract::command::TargetRef::Identified {
                    id: seen.expect("the other jet seen"),
                },
            },
        });
        assert_eq!(ack.error, None);
    }
    battle
}

/// How far apart the two jets are over the ground (their heights differ by
/// their dogfight bands).
fn apart(battle: &sim::battle::Battle) -> f64 {
    let p = |id| battle.unit(UnitId(id)).unwrap().position.xy();
    (p(0) - p(1)).length()
}

/// The test jet flown like the shipped jets: their speed, wide turn, slow
/// roll-in and long sight, shorter behind than ahead, the loop of passes at
/// its widest.
fn slow_turning_rules(weapons: &[&str]) -> serde_json::Value {
    let mut rules = crate::attack_runs::run_rules(weapons);
    let shipped = sim::fixtures::test_rules();
    rules["air"]["fixed_wing"]["roll_in_s"] = shipped["air"]["fixed_wing"]["roll_in_s"].clone();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({
            "mobility": { "air": { "cruise_kmh": 650, "turn_deg_s": 30 } },
            "sensors": {
                "ground_m": 3000, "low_air_m": 3000, "high_air_m": 3000,
                "sight_shape": { "front": 1.0, "side": 0.8, "rear": 0.5 }
            }
        }),
    );
    rules
}

/// Whether one of the two jets is down within `seconds`.
fn one_downed(battle: &mut sim::battle::Battle, seconds: u64) -> bool {
    let hz = u64::from(battle.rules().tick_hz);
    let alive = |b: &sim::battle::Battle, id| b.unit(UnitId(id)).unwrap().alive();
    (0..seconds * hz).any(|_| {
        battle.step();
        !alive(battle, 0) || !alive(battle, 1)
    })
}

/// Two jets armed with guns alone meet head-on, gun each other on the way
/// in, flash past and come round again until one goes down: tight-turning
/// jets and wide-turning ones alike.
#[test]
fn gun_armed_jets_shoot_each_other_down() {
    let mut tight = duel(&["air_gun"], None);
    assert!(
        one_downed(&mut tight, 240),
        "tight-turning jets never gunned one down"
    );
    let mut wide = duel_with(slow_turning_rules(&["air_gun"]), None);
    assert!(
        one_downed(&mut wide, 240),
        "wide-turning jets never gunned one down"
    );
}

/// A head-on merge is a pass: the jets fly at each other, pass close beside
/// one another rather than through, open out, and turn back for another
/// pass, again and again.
#[test]
fn head_on_jets_pass_beside_each_other_and_come_round_again() {
    let mut battle = duel(&["air_gun"], Some(1_000_000));
    let separation = battle.rules().air.separation_m;
    let hz = u64::from(battle.rules().tick_hz);
    let mut passes = Vec::new();
    let (mut last, mut closing) = (apart(&battle), true);
    for _ in 0..180 * hz {
        battle.step();
        let now = apart(&battle);
        if closing && now > last {
            passes.push(last);
        }
        closing = now < last;
        last = now;
    }
    assert!(
        passes.len() >= 3,
        "they came round for too few passes: {passes:?}"
    );
    for (k, d) in passes.iter().enumerate() {
        assert!(
            *d >= separation * 0.5,
            "pass {k} flew through the other jet: {d} m"
        );
        assert!(
            *d < separation * 4.0,
            "pass {k} was no merge: {d} m apart at the closest"
        );
    }
}

/// Jets armed with guns and air-to-air missiles use both on their passes:
/// the missiles from farther out (a seeker fires within its cone under any
/// flight law), and the guns, which fire only nose-on, as they close.
#[test]
fn jets_with_guns_and_missiles_use_both_on_their_passes() {
    let mut battle = duel(&["air_gun", "air_missile"], Some(1_000_000));
    let hz = u64::from(battle.rules().tick_hz);
    for _ in 0..90 * hz {
        battle.step();
    }
    for id in [0, 1] {
        let jet = battle.unit(UnitId(id)).unwrap();
        let [gun, missiles] = [0, 1].map(|m| jet.mounts[m].shots);
        assert!(
            gun > 0 && missiles > 0,
            "jet {id} fired guns {gun}, missiles {missiles}"
        );
    }
}

/// A jet with an enemy on its tail turns to face it instead of flying on
/// straight while it is shot at, and the fight that follows, guns only,
/// ends with one jet down.
#[test]
fn a_jet_with_an_enemy_on_its_tail_turns_to_face_it() {
    use crate::attack_runs::{run_field, run_rules, RUN_START};
    let [x, y] = RUN_START;
    let mut battle = run_field(
        run_rules(&["air_gun"]),
        json!([
            { "side": "blue", "kind": "test_jet", "position": [x + 300.0, y], "yaw": 0.0 },
            { "side": "red", "kind": "test_jet", "position": [x, y], "yaw": 0.0 },
        ]),
    );
    let mut seq = [0, 0];
    for (side, unit) in [(Side::Blue, 0), (Side::Red, 1)] {
        let seen = (0..300).find_map(|_| {
            let id = battle.observe(side).identified.first().map(|t| t.id);
            if id.is_none() {
                battle.step();
            }
            id
        });
        seq[side.index()] += 1;
        let ack = battle.accept(contract::command::CommandEnvelope {
            side,
            player: None,
            seq: seq[side.index()],
            queued: false,
            order: contract::command::Order::Attack {
                units: vec![UnitId(unit)],
                target: contract::command::TargetRef::Identified {
                    id: seen.expect("seen"),
                },
            },
        });
        assert_eq!(ack.error, None);
    }
    let hz = u64::from(battle.rules().tick_hz);
    let turned = (0..10 * hz).any(|_| {
        battle.step();
        battle.unit(UnitId(0)).unwrap().yaw.cos() < 0.0
    });
    assert!(
        turned,
        "the jet in front flew on straight with an enemy on its tail"
    );
    // And the fight that follows ends with one gunned down, not a circle.
    assert!(
        one_downed(&mut battle, 240),
        "neither jet gunned the other down"
    );
}

/// Jets that turn as slowly as the shipped ones (a wide turn) hold their
/// fight pass after pass, though coming round carries them past the break
/// distance, and open out far enough that each comes back at the other
/// with its gun bearing: it fires on most passes.
#[test]
fn slow_turning_jets_hold_their_fight_and_gun_each_other_pass_after_pass() {
    let mut battle = duel_with(slow_turning_rules(&["air_gun"]), Some(1_000_000));
    let hz = u64::from(battle.rules().tick_hz);
    let linked = |b: &sim::battle::Battle| {
        [0, 1].iter().all(|&id| {
            b.unit(UnitId(id))
                .unwrap()
                .air
                .unwrap()
                .dogfight_target
                .is_some()
        })
    };
    let formed = (0..30 * hz).any(|_| {
        battle.step();
        linked(&battle)
    });
    assert!(formed, "the dogfight never formed");
    let shots =
        |b: &sim::battle::Battle| [0, 1].map(|id| b.unit(UnitId(id)).unwrap().mounts[0].shots);
    let (mut last, mut closing, mut passes) = (apart(&battle), true, 0);
    let (mut before, mut gunned) = (shots(&battle), [0; 2]);
    for _ in 0..150 * hz {
        battle.step();
        assert!(
            linked(&battle),
            "the dogfight broke at {} m",
            apart(&battle)
        );
        let now = apart(&battle);
        if closing && now > last {
            passes += 1;
            let fired = shots(&battle);
            for jet in 0..2 {
                gunned[jet] += usize::from(fired[jet] > before[jet]);
            }
            before = fired;
        }
        closing = now < last;
        last = now;
    }
    assert!(passes >= 3, "only {passes} passes");
    for (jet, &n) in gunned.iter().enumerate() {
        assert!(
            2 * n > passes,
            "jet {jet} fired its gun on only {n} of {passes} passes"
        );
    }
}
