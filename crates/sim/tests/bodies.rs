//! Bodies (Q14, Q19, Q28): one body table read column by column, the
//! kinematic shove (Q2), and what each side learns of a shove (L1–L3).
use crate::common;

use contract::ids::{Side, UnitId};
use contract::map::{MoverClass, PropKind};
use contract::observation::MoveState;
use contract::scenario::{Rules, ScenarioDefinition, UnitKind};
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
        "size": [width, 60], "height_grid_m": 4, "slope_cutoff_deg": 35,
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

fn kind_name(kind: PropKind) -> String {
    serde_json::to_value(kind)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

/// Q2–Q4: a vehicle shoves a body only when its push class is strictly
/// heavier than the body's weight class. A body it cannot shove stops it:
/// its hull never enters the body, and it never gets past.
#[test]
fn a_vehicle_shoves_only_bodies_strictly_lighter_than_its_push_class() {
    let r = rules();
    for mover in [UnitKind::Jeep, UnitKind::Supply, UnitKind::Tank] {
        for body in [
            PropKind::Crate,
            PropKind::Fence,
            PropKind::Sandbags,
            PropKind::Tooth,
            PropKind::JeepWreck,
            PropKind::TankWreck,
        ] {
            // A wall across the map with a 9.6 m gate; the body fills most of it.
            let at = [55.0, 30.0];
            let map = json!({
                "size": [140, 60], "height_grid_m": 4, "slope_cutoff_deg": 35,
                "props": [
                    { "kind": "wall", "center": [55, 12.6], "yaw": 0, "half_extents": [5, 12.6, 1.5] },
                    { "kind": "wall", "center": [55, 47.4], "yaw": 0, "half_extents": [5, 12.6, 1.5] },
                    { "kind": kind_name(body), "center": at, "yaw": 0, "half_extents": [0.8, 3.5, 0.6] },
                ],
            })
            .to_string();
            let mover_name = serde_json::to_value(mover).unwrap();
            let setup = common::scenario_with(
                &map,
                json!([vehicle("blue", mover_name.as_str().unwrap(), [15.0, 30.0])]),
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
            let shoves = r.bodies[&mover]
                .push_class
                .unwrap()
                .pushes(r.props[&body].weight_class);
            let moved = (b.world().prop(BODY).unwrap().center - v2(at[0], at[1])).length();
            let passed = b.unit(UnitId(0)).unwrap().position.x > at[0] + 5.0;
            let case = format!("{mover:?} against {body:?}");
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

/// A truck shoves a crate down the lane while a red squad watches from
/// `red_at`. The crate's id, and the battle once the crate has come to rest.
fn watched_shove(red_at: [f64; 2]) -> (Battle, V2) {
    let setup = common::scenario_with(
        &lane(
            900.0,
            json!({ "kind": "crate", "center": [55, 30], "yaw": 0, "half_extents": [0.8, 0.8, 0.6] }),
        ),
        json!([
            vehicle("blue", "supply", [15.0, 30.0]),
            { "side": "red", "kind": "rifle", "position": red_at, "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([drive("blue", 0, [110.0, 30.0])]),
    );
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
    assert_eq!(hidden.navigation_revision(Side::Red), 0);
    assert!(seen.navigation_revision(Side::Red) > 0);
}

/// L3: a shove is replayed exactly, and a body's pose is battle state.
#[test]
fn shoves_replay_and_poses_are_in_the_digest() {
    let (b, _) = watched_shove([150.0, 30.0]);
    let setup = common::scenario_with(
        &lane(
            900.0,
            json!({ "kind": "crate", "center": [55, 30], "yaw": 0, "half_extents": [0.8, 0.8, 0.6] }),
        ),
        json!([
            vehicle("blue", "supply", [15.0, 30.0]),
            { "side": "red", "kind": "rifle", "position": [150, 30], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([drive("blue", 0, [110.0, 30.0])]),
    );
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

/// Q28: a test-only smoke row (the crate's row, patched): blocks nobody,
/// stops no rounds, gives no cover, occludes, and lasts 10 s. Each column
/// acts alone.
fn smoke(units: Value, scripts: Value) -> ScenarioDefinition {
    let mut rules = common::village();
    rules["props"]["crate"] = json!({
        "blocks": { "infantry": false, "vehicle": false }, "stops_rounds": false, "occludes": true,
        "weight_class": "immovable", "lifetime_s": 10,
    });
    let map = json!({
        "size": [200, 100], "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": [{ "kind": "crate", "center": [100, 50], "yaw": 0, "half_extents": [3, 20, 5] }],
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
        .find(|w| w.name == "rifle")
        .unwrap()
        .def;
    for w in &arsenal.weapons {
        let hurts = sim::weapons::can_damage(&w.def, UnitKind::Jeep, &r.health);
        let heavier = w.def.penetration > rifle.penetration;
        assert_eq!(hurts, heavier, "{}", w.name);
    }
    let range = r.sensors.jeep_ground_m;
    for dx in [-0.9 * range, 0.9 * range] {
        let setup = common::scenario(
            &json!({ "size": [1200, 200], "height_grid_m": 8, "slope_cutoff_deg": 35 }).to_string(),
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
        &json!({ "size": [400, 200], "height_grid_m": 4, "slope_cutoff_deg": 35 }).to_string(),
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
    assert_eq!(Some(wreck.kind), r.bodies[&UnitKind::Jeep].wreck);
    assert!(r.bodies[&UnitKind::Tank]
        .push_class
        .unwrap()
        .pushes(wreck.body.weight_class));
}
