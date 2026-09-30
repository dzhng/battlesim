//! Bodies (Q14, Q19, Q28): the prop types' body rows read column by column, the
//! kinematic shove (Q2), and what each side learns of a shove (L1–L3).
use crate::common;

use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::MoveState;
use contract::scenario::{Rules, ScenarioDefinition};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::ground::GroundLayer;
use sim::math::{v2, v3, V2};

fn rules() -> Rules {
    serde_json::from_value(common::village()).unwrap()
}

fn run(b: &mut Battle, seconds: f64) {
    for _ in 0..(seconds * b.rules().tick_hz as f64).round() as u64 {
        b.step();
    }
}

/// A 9 m lane between two tall walls from x = 30 to 90, and `body` in it.
fn lane(width: f64, body: Value) -> String {
    json!({
        "size": [width, 60], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": [
            { "kind": "wall", "center": [60, 25.2], "yaw": 0, "half_extents": [30, 0.4, 1.5] },
            { "kind": "wall", "center": [60, 34.8], "yaw": 0, "half_extents": [30, 0.4, 1.5] },
            body,
        ],
    })
    .to_string()
}

/// The body in the lane is the third authored prop.
const BODY: u32 = 2;

fn drive(side: &str, unit: u32, goal: [f64; 2]) -> Value {
    json!({ "tick": 1, "side": side, "order":
        { "kind": "move", "units": [unit], "gesture": unit + 1, "goal": goal, "route": "shortest" } })
}

fn vehicle(side: &str, kind: &str, at: [f64; 2]) -> Value {
    json!({ "side": side, "kind": kind, "position": at, "engagement": "return_fire_only" })
}

/// Q2–Q4: a vehicle shoves a body only when its push class is strictly
/// heavier than the body's weight class. A body it cannot shove stops it:
/// its hull never enters the body, and it never gets past.
#[test]
fn a_vehicle_shoves_only_bodies_strictly_lighter_than_its_push_class() {
    let r = rules();
    for mover in ["jeep", "supply", "tank"] {
        for body in [
            "crate",
            "fence",
            "sandbags",
            "tooth",
            "jeep_wreck",
            "tank_wreck",
        ] {
            // A wall across the map with a 9.6 m gate; the body fills most of it.
            let at = [55.0, 30.0];
            let map = json!({
                "size": [140, 60], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                "props": [
                    { "kind": "wall", "center": [55, 12.6], "yaw": 0, "half_extents": [5, 12.6, 1.5] },
                    { "kind": "wall", "center": [55, 47.4], "yaw": 0, "half_extents": [5, 12.6, 1.5] },
                    { "kind": body, "center": at, "yaw": 0, "half_extents": [0.8, 3.5, 0.6] },
                ],
            })
            .to_string();
            let setup = common::scenario_with(
                &map,
                json!([vehicle("blue", mover, [15.0, 30.0])]),
                json!([]),
                json!([drive("blue", 0, [125.0, 30.0])]),
            );
            let mut b = Battle::new(&setup, 1);
            let mut entered = false;
            for _ in 0..(30.0 * r.tick_hz as f64) as u64 {
                b.step();
                let hull = b.unit(UnitId(0)).unwrap().hull_box().unwrap();
                let prop = b.world().prop(BODY).unwrap().footprint();
                entered |= hull.separation(&prop).is_some_and(|v| v.length() > 0.05);
            }
            let shoves = common::hull(mover)
                .push_class
                .pushes(r.catalog.props().by_id(body).body.weight_class);
            let moved = (b.world().prop(BODY).unwrap().center - v2(at[0], at[1])).length();
            let passed = b.unit(UnitId(0)).unwrap().position.x > at[0] + 5.0;
            let case = format!("{mover} against {body}");
            if shoves {
                assert!(moved > 2.0, "{case}: shoved {moved:.2} m");
                assert!(passed, "{case}: got through");
            } else {
                assert_eq!(moved, 0.0, "{case}: nothing up-class moves");
                assert!(
                    !entered && !passed,
                    "{case}: stopped at the body (entered {entered}, passed {passed})"
                );
            }
        }
    }
}

/// A truck shoves a crate along a ridge while red watches from `red_at`.
fn shove_setup(red_at: [f64; 2]) -> ScenarioDefinition {
    let mut map: Value = serde_json::from_str(&lane(
        900.0,
        json!({ "kind": "crate", "center": [55, 30], "yaw": 0, "half_extents": [0.8, 0.8, 0.6] }),
    ))
    .unwrap();
    map["relief"] =
        json!([{ "kind": "ridge", "center": [100, 100], "peak_m": 12, "radius_m": 200 }]);
    common::scenario_with(
        &map.to_string(),
        json!([
            vehicle("blue", "supply", [15.0, 30.0]),
            { "side": "red", "kind": "rifle", "position": red_at, "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([drive("blue", 0, [110.0, 30.0])]),
    )
}

fn watched_shove(red_at: [f64; 2]) -> (Battle, V2) {
    let setup = shove_setup(red_at);
    let mut b = Battle::new(&setup, 1);
    run(&mut b, 35.0);
    assert_eq!(b.unit(UnitId(0)).unwrap().state, MoveState::Idle);
    let now = b.world().prop(BODY).unwrap().center;
    assert!((now - v2(55.0, 30.0)).length() > 2.0, "the truck shoved it");
    (b, now)
}

/// Where `side` believes the crate stands: its authored place unless the
/// side has seen it moved (L1).
fn believed(b: &Battle, side: Side) -> V2 {
    b.observe(side)
        .known_props
        .iter()
        .find(|p| p.replaces == Some(BODY))
        .map_or(v2(55.0, 30.0), |p| v2(p.center[0], p.center[1]))
}

/// L1, L2 (metamorphic): the same shove, watched or not. A side that saw
/// the crate come to rest knows where it lies and replanned; a side beyond
/// its sight still believes it where it stood, and its planning never
/// changed.
#[test]
fn a_side_that_did_not_see_a_shove_keeps_the_old_pose() {
    let (hidden, now) = watched_shove([880.0, 30.0]);
    let (seen, same) = watched_shove([150.0, 30.0]);
    assert!((now - same).length() < 1e-9, "the observer changes nothing");
    for b in [&hidden, &seen] {
        assert!(
            (believed(b, Side::Blue) - now).length() < 1e-9,
            "the pusher knows"
        );
        assert!(
            b.navigation_revision(Side::Blue) > 0,
            "and replanned with it"
        );
    }
    assert!((believed(&seen, Side::Red) - now).length() < 1e-9);
    assert_eq!(believed(&hidden, Side::Red), v2(55.0, 30.0));
    let initial_z = hidden.world().height_at(55.0, 30.0).unwrap();
    let moved_z = hidden.world().prop(BODY).unwrap().base_z;
    assert!(
        (moved_z - initial_z).abs() > 0.01,
        "the shove changes elevation"
    );
    for (battle, expected) in [(&hidden, initial_z), (&seen, moved_z)] {
        let prop = battle
            .observe(Side::Red)
            .known_props
            .iter()
            .find(|p| p.replaces == Some(BODY))
            .unwrap();
        assert_eq!(
            prop.base_z, expected,
            "the complete last-seen pose stays together"
        );
    }
    assert_eq!(hidden.navigation_revision(Side::Red), 0);
    assert!(seen.navigation_revision(Side::Red) > 0);
}

/// L3: a shove is replayed exactly, and a body's pose is battle state.
#[test]
fn shoves_replay_and_poses_are_in_the_digest() {
    let (b, _) = watched_shove([150.0, 30.0]);
    let setup = shove_setup([150.0, 30.0]);
    let mut again = Battle::from_replay(&setup, &b.replay()).unwrap();
    run(&mut again, 35.0);
    assert_eq!(again.digest(), b.digest());
    let at = |y: f64| {
        let map = lane(
            160.0,
            json!({ "kind": "crate", "center": [55, y], "yaw": 0, "half_extents": [0.8, 0.8, 0.6] }),
        );
        Battle::new(&common::scenario(&map, json!([]), json!([])), 1).digest()
    };
    assert_ne!(at(30.0), at(30.01));
}

/// Q28: a test-only smoke prop type: blocks nobody, stops no rounds, gives
/// no cover, occludes, and lasts 10 s. Each column acts alone.
fn smoke(units: Value, scripts: Value) -> ScenarioDefinition {
    let mut rules = common::village();
    rules["catalog"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "props": { "smoke": {
        "body": { "blocks": { "infantry": false, "vehicle": false }, "stops_rounds": false,
            "occludes": true, "weight_class": "immovable", "lifetime_s": 10 },
        "appearance": { "drawn_by": "smoke" },
    } } }));
    let map = json!({
        "size": [200, 100], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": [{ "kind": "smoke", "center": [100, 50], "yaw": 0, "half_extents": [3, 20, 5] }],
    });
    serde_json::from_value(json!({
        "map": map, "rules": rules, "units": units, "events": [], "scripts": scripts,
    }))
    .unwrap()
}

#[test]
fn a_smoke_row_blocks_nobody_but_hides_while_it_lasts() {
    // Movers walk and drive straight through it.
    let mut b = Battle::new(
        &smoke(
            json!([
                { "side": "blue", "kind": "rifle", "position": [84, 50], "engagement": "return_fire_only" },
                vehicle("blue", "tank", [72.0, 38.0]),
            ]),
            json!([
                drive("blue", 0, [140.0, 50.0]),
                drive("blue", 1, [140.0, 38.0])
            ]),
        ),
        1,
    );
    let cloud = b.world().props().next().unwrap().footprint();
    let (mut soldier_in, mut hull_in) = (false, false);
    for _ in 0..(8.0 * b.rules().tick_hz as f64) as u64 {
        b.step();
        let u = b.unit(UnitId(0)).unwrap();
        soldier_in |= u.member_positions().any(|p| cloud.contains(p.xy(), 0.0));
        hull_in |= cloud.contains(b.unit(UnitId(1)).unwrap().position.xy(), 0.0);
    }
    assert!(soldier_in && hull_in, "nobody walks round it");
    // Rounds fly through it, but it hides what lies behind it.
    let (a, z) = (v3(60.0, 50.0, 1.5), v3(140.0, 50.0, 1.5));
    assert!(b.world().segment_clear(a, z));
    assert!(b.world().raycast(a, v3(1.0, 0.0, 0.0), 80.0).is_none());
    assert!(!b.world().sight_clear(a, z));
    // It gives no cover to a soldier behind it.
    let r = rules();
    let ground = GroundLayer::new(200.0, 100.0, &r.ground);
    assert_eq!(
        sim::cover::at(b.world(), &ground, &[], &r, v2(96.5, 50.0), v2(140.0, 50.0)),
        None
    );
}

#[test]
fn a_smoke_row_hides_from_sensing_and_the_fog_until_it_expires() {
    let mut b = Battle::new(
        &smoke(
            json!([
                { "side": "blue", "kind": "rifle", "position": [60, 50], "engagement": "return_fire_only" },
                { "side": "red", "kind": "rifle", "position": [140, 50], "engagement": "return_fire_only" },
            ]),
            json!([]),
        ),
        1,
    );
    run(&mut b, 5.0);
    let o = b.observe(Side::Blue);
    assert!(o.identified.is_empty(), "the smoke hides the red squad");
    assert!(!o.ground_visibility.visible(140.0, 50.0));
    let (world, revision) = (
        b.world().obstacle_revision(),
        b.navigation_revision(Side::Blue),
    );
    run(&mut b, 6.0);
    assert_eq!(b.world().props().count(), 0, "it lasted its 10 s");
    assert!(b.world().obstacle_revision() > world);
    assert!(b.navigation_revision(Side::Blue) > revision);
    let o = b.observe(Side::Blue);
    assert!(
        !o.identified.is_empty(),
        "the red squad is seen once it clears"
    );
    assert!(o.ground_visibility.visible(140.0, 50.0));
}

/// The jeep (Q3): open-topped, so it sees as far behind as ahead; lightly
/// armoured, so rifle rounds stop and every heavier row gets through.
#[test]
fn a_jeep_sees_all_round_and_only_rifles_cannot_hurt_it() {
    let r = rules();
    let arsenal = sim::weapons::Arsenal::new(&r);
    let rifle = &arsenal
        .weapons
        .iter()
        .find(|w| w.id == "rifle")
        .unwrap()
        .def;
    for w in &arsenal.weapons {
        let hurts = sim::weapons::can_damage(&w.def, Some(&common::hull("jeep").armor));
        let heavier = w.def.penetration > rifle.penetration;
        assert_eq!(hurts, heavier, "{}", w.id);
    }
    let range = r.catalog.by_id("jeep").sensors.ground_m;
    for dx in [-0.9 * range, 0.9 * range] {
        let setup = common::scenario(
            &json!({ "size": [1200, 200], "fog_cell_m": 8, "height_grid_m": 8, "slope_cutoff_deg": 35 }).to_string(),
            json!([
                vehicle("blue", "jeep", [600.0, 100.0]),
                { "side": "red", "kind": "rifle", "position": [600.0 + dx, 100], "engagement": "return_fire_only" },
            ]),
            json!([]),
        );
        let mut b = Battle::new(&setup, 1);
        b.step();
        assert!(
            !b.observe(Side::Blue).identified.is_empty(),
            "seen at {dx} m"
        );
    }
}

/// A destroyed jeep leaves a wreck a tank shoves (Q3, Q24): its own
/// wreck row, as light as the jeep was.
#[test]
fn a_destroyed_jeep_leaves_a_light_wreck() {
    let r = rules();
    let setup = common::scenario(
        &json!({ "size": [400, 200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
            .to_string(),
        json!([
            vehicle("blue", "jeep", [200.0, 100.0]),
            { "side": "red", "kind": "tank", "position": [320, 100], "yaw": std::f64::consts::PI },
        ]),
        json!([]),
    );
    let mut b = Battle::new(&setup, 1);
    for _ in 0..(40 * r.tick_hz) {
        b.step();
        if !b.unit(UnitId(0)).unwrap().alive() {
            break;
        }
    }
    let wreck = b
        .world()
        .props()
        .find(|p| p.blocks(MoverClass::Vehicle))
        .expect("the jeep left a wreck");
    assert_eq!(b.world().types().id(wreck.kind), common::hull("jeep").wreck);
    assert!(common::hull("tank")
        .push_class
        .pushes(wreck.body.weight_class));
}

/// A new prop type is one catalog entry: a vehicle whose `wreck` names a
/// type added in a document of its own leaves that type, with its own body
/// row, and nothing in code knows its id.
#[test]
fn a_wreck_is_whatever_prop_type_its_vehicle_names() {
    let mut rules = common::village();
    rules["catalog"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "props": {
        "burnt_out_jeep": { "extends": "wreck",
            "body": { "weight_class": "light", "cover_tier": "light", "hp": 35, "armor": 0.25 },
            "destroyed": "removed" },
    } }));
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "jeep",
        json!({ "body": { "hull": { "wreck": "burnt_out_jeep" } } }),
    );
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [400, 200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            vehicle("blue", "jeep", [200.0, 100.0]),
            { "side": "red", "kind": "tank", "position": [320, 100], "yaw": std::f64::consts::PI },
        ],
        "events": [], "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    for _ in 0..(40 * setup.rules.tick_hz) {
        b.step();
        if !b.unit(UnitId(0)).unwrap().alive() {
            break;
        }
    }
    let wreck = b
        .world()
        .props()
        .find(|p| p.blocks(MoverClass::Vehicle))
        .expect("the jeep left a wreck");
    assert_eq!(b.world().types().id(wreck.kind), "burnt_out_jeep");
    assert_eq!((wreck.body.hp, wreck.body.armor), (Some(35.0), 0.25));
}
