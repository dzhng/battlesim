//! Bodies (Q14, Q19, Q28): the prop types' body rows read column by column, the
//! kinematic shove (Q2), and what each side learns of a shove (L1–L3).
use crate::common;

use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::MoveState;
use contract::scenario::{PushClass, Rules, ScenarioDefinition, WeightClass};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::ground::GroundLayer;
use sim::math::{v2, v3, V2};

fn rules() -> Rules {
    serde_json::from_value(common::game()).unwrap()
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
    for mover in ["test_jeep", "test_supply", "test_tank"] {
        for body in [
            "crate",
            "fence",
            "sandbags",
            "tooth",
            "light_wreck",
            "heavy_wreck",
            "lamp",
            "bench",
            "bollard",
            "bins",
            "hydrant",
            "utility_box",
            "scooter",
            "planter",
            "parked_car",
            "car_wreck",
            "jersey_barrier",
            "bus_shelter",
            "scaffold",
            "heras_fence",
            "skip_bin",
            "pallet_stack",
            "site_cabin",
            "road_barrier",
            "log",
            "boulder",
        ] {
            // A wall across the map with a 9.6 m gate; the body fills most of it.
            let at = [55.0, 30.0];
            let map = json!({
                "size": [140, 60], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                "props": [
                    { "kind": "wall", "center": [55, 12.6], "yaw": 0, "half_extents": [5, 12.6, 1.5] },
                    { "kind": "wall", "center": [55, 47.4], "yaw": 0, "half_extents": [5, 12.6, 1.5] },
                    // A wreck names the unit it was; any test tank's will do.
                    { "kind": body, "center": at, "yaw": 0, "half_extents": [0.8, 3.5, 0.6],
                      "wreck_of": (["light_wreck", "heavy_wreck"].contains(&body)).then_some("test_tank") },
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
                let hull = b.unit(UnitId(0)).unwrap().ground_footprint().unwrap();
                let prop = b.world().prop(BODY).unwrap().footprint();
                entered |= hull.separation(&prop).is_some_and(|v| v.length() > 0.05);
            }
            let weight = r.catalog.props().by_id(body).body.weight_class;
            let shoves = match common::hull(mover).push_class {
                PushClass::None | PushClass::Light => false,
                PushClass::Medium => matches!(weight, WeightClass::Light),
                PushClass::Heavy => matches!(weight, WeightClass::Light | WeightClass::Medium),
                PushClass::SuperHeavy => matches!(
                    weight,
                    WeightClass::Light | WeightClass::Medium | WeightClass::Heavy
                ),
            };
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

/// A fence line is panels laid end to end, each touching the next by a few
/// millimetres. A truck that can shove a panel, striking one a little off
/// square, slides it along its neighbour's end rather than into it: that is
/// no chain shove, and the truck drives through the line.
#[test]
fn a_vehicle_shoves_a_panel_that_only_touches_its_neighbour() {
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        json!({ "body": { "hull": { "push_class": "medium" } } }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "fence",
        json!({ "body": { "weight_class": "light" } }),
    );
    // The line closes a gate in a thin wall, a few degrees off square to the
    // truck, so the slide that clears the hull edges each panel along the
    // line toward its neighbour; the joint overlaps by 5 mm. Past the gate,
    // a shoved panel is clear of the wall.
    let tilt: f64 = -0.04;
    let along = v2(-tilt.sin(), tilt.cos());
    let half_len = 2.75;
    let first = v2(55.0, 30.0) - along * (half_len - 0.0025);
    let second = first + along * (2.0 * half_len - 0.005);
    let panel = |at: V2| {
        json!({ "kind": "fence", "center": [at.x, at.y], "yaw": tilt,
                "half_extents": [0.05, half_len, 0.6] })
    };
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [140, 60], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "props": [
                { "kind": "wall", "center": [55, 12.0], "yaw": 0, "half_extents": [0.5, 12.0, 1.5] },
                { "kind": "wall", "center": [55, 48.0], "yaw": 0, "half_extents": [0.5, 12.0, 1.5] },
                panel(first),
                panel(second),
            ] },
        "rules": rules,
        "units": [vehicle("blue", "test_supply", [15.0, 28.0])],
        "events": [],
        "scripts": [drive("blue", 0, [125.0, 28.0])],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    run(&mut b, 30.0);

    let unit = b.unit(UnitId(0)).unwrap();
    assert!(
        unit.position.x > 65.0,
        "it drives through the fence line: at {:?}, {:?}",
        unit.position.xy(),
        unit.state
    );
    let moved = (b.world().prop(BODY).unwrap().center - first).length();
    assert!(moved > 1.0, "the panel it struck was shoved {moved:.2} m");
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
            vehicle("blue", "test_supply", [15.0, 30.0]),
            { "side": "red", "kind": "test_rifle", "position": red_at, "engagement": "return_fire_only" },
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

/// A shoved body is drawn where it stands each tick, ahead of the knowledge
/// that re-learns it only in steps: to its pusher always, to a side that
/// watches it, never to a side that can't see it, and to no one once it lies
/// still. The pusher still drives on past it to its goal.
#[test]
fn a_shove_is_published_each_tick_to_the_sides_that_see_it() {
    for (red_at, red_sees) in [([150.0, 30.0], true), ([880.0, 30.0], false)] {
        let mut b = Battle::new(&shove_setup(red_at), 1);
        let (mut moved, mut red_drawn) = (0, 0);
        let mut was = b.world().prop(BODY).unwrap().center;
        for _ in 0..(35.0 * b.rules().tick_hz as f64) as u64 {
            b.step();
            let now = b.world().prop(BODY).unwrap().center;
            if now != was {
                moved += 1;
                let drawn = &b.observe(Side::Blue).prop_display_poses;
                assert_eq!(drawn.len(), 1, "the pusher draws what it moved");
                assert_eq!(drawn[0].center, [now.x, now.y]);
            }
            red_drawn += usize::from(!b.observe(Side::Red).prop_display_poses.is_empty());
            was = now;
        }
        assert!(moved > 10, "the truck shoved the crate along");
        assert_eq!(red_drawn > 0, red_sees, "red at {red_at:?}");
        let truck = b.unit(UnitId(0)).unwrap();
        assert_eq!(truck.state, MoveState::Idle);
        assert!((truck.position.xy() - v2(110.0, 30.0)).length() < 3.0);
        b.step();
        assert!(b.observe(Side::Blue).prop_display_poses.is_empty());
    }
}

/// A tank shoving a parked car out of a row holds its heading (Q13): the car
/// it is pressed against moves with it, so it never re-plans round the car
/// and swings from side to side. It drives at the row on a slant, through
/// one car's flank, to a goal past it.
#[test]
fn a_tank_shoving_a_car_holds_its_heading() {
    let mut b = Battle::new(&car_row(), 1);
    let car = 2;
    let parked = b.world().prop(car).unwrap().center;
    let mut yaws = Vec::new();
    for _ in 0..30 * b.rules().tick_hz as u64 {
        b.step();
        let shoved = (b.world().prop(car).unwrap().center - parked).length() > 0.1;
        let tank = b.unit(UnitId(0)).unwrap();
        if shoved && tank.state != MoveState::Idle {
            yaws.push(tank.yaw.to_degrees());
        }
    }
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(
        (tank.position.xy() - v2(27.0, 38.0)).length() < 0.5,
        "the tank reaches its goal: {:?}",
        tank.position.xy()
    );
    assert!(
        yaws.len() > 30,
        "it shoves the car for a while: {} ticks",
        yaws.len()
    );
    let (lo, hi) = yaws
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), y| (lo.min(*y), hi.max(*y)));
    assert!(
        hi - lo < 2.0,
        "its heading swings {lo:.1}..{hi:.1} degrees while it shoves"
    );
}

/// A tank driving at a row of parked cars on a slant, through one car's flank
/// (prop 2), to a goal past it.
fn car_row() -> ScenarioDefinition {
    car_row_to([27.0, 38.0], json!([]))
}

fn car_row_to(goal: [f64; 2], events: Value) -> ScenarioDefinition {
    let cars: Vec<Value> = [15.0, 20.0, 25.0, 30.0, 35.0]
        .map(|x| json!({ "kind": "parked_car", "center": [x, 30.0], "yaw": 0, "half_extents": [2.1, 0.9, 0.75] }))
        .to_vec();
    let map = json!({
        "size": [60, 70], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": cars,
    })
    .to_string();
    common::scenario_with(
        &map,
        json!([vehicle("blue", "test_tank", [22.0, 5.0])]),
        events,
        json!([drive("blue", 0, goal)]),
    )
}

/// A body that turns up on a pusher's way while it shoves another is still
/// weighed against a detour (Q13): a car left across the road beyond the row
/// while the tank is pressed against the one it shoves, the tank drives
/// round it, never through where it was left.
#[test]
fn a_tank_shoving_a_car_still_goes_round_one_left_in_its_way() {
    let at = v2(28.5, 50.0);
    let left = json!([{ "tick": 180, "add_prop":
        { "kind": "parked_car", "center": [at.x, at.y], "yaw": 0, "half_extents": [2.1, 0.9, 0.75] } }]);
    let mut b = Battle::new(&car_row_to([30.0, 62.0], left), 1);
    let parked = b.world().prop(2).unwrap().center;
    let mut shoving_when_left = false;
    let mut box_left = None;
    let mut nearest = f64::INFINITY;
    for _ in 0..40 * b.rules().tick_hz as u64 {
        b.step();
        let tank = b.unit(UnitId(0)).unwrap().position.xy();
        if box_left.is_none() {
            box_left = b
                .world()
                .props()
                .find(|p| p.center == at)
                .map(|p| p.footprint());
            if box_left.is_some() {
                let shoved = b.world().prop(2).unwrap();
                shoving_when_left =
                    shoved.center != parked && shoved.footprint().distance(tank) < 4.0;
            }
        }
        if let Some(left) = box_left {
            nearest = nearest.min(left.distance(tank));
        }
    }
    assert!(
        shoving_when_left,
        "the car is left while the tank shoves the first"
    );
    assert!(
        nearest > 0.0,
        "the tank drives round the car left in its way, never through where it stood"
    );
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(
        (tank.position.xy() - v2(30.0, 62.0)).length() < 0.5,
        "the tank reaches its goal: {:?}",
        tank.position.xy()
    );
}

/// A hull grinds the car it shoves where they meet: on the car's side facing
/// the hull, never its far side, halfway up the lower of the two.
#[test]
fn a_hull_grinds_the_car_it_shoves_where_they_meet() {
    let mut b = Battle::new(&car_row(), 1);
    let hull_half = b.unit(UnitId(0)).unwrap().hull.unwrap().z;
    let flank = b.world().prop(2).unwrap().base_z + hull_half.min(0.75);
    let mut grinds = 0;
    for _ in 0..30 * b.rules().tick_hz as u64 {
        b.step();
        let hull = b.unit(UnitId(0)).unwrap().ground_footprint().unwrap();
        for c in &b.observe(Side::Blue).grinds {
            let [x, y, z] = c.point;
            assert!(
                hull.distance(v2(x, y)) < 0.25,
                "the grind {:.1} m from the hull",
                hull.distance(v2(x, y))
            );
            assert!(
                (z - flank).abs() < 1e-9,
                "the grind at {z:.2} m, not {flank:.2}"
            );
            grinds += 1;
        }
    }
    assert!(grinds > 30, "the hull grinds the car for {grinds} ticks");
}

/// Q28: a test-only smoke prop type: blocks nobody, stops no rounds, gives
/// no cover, occludes, and lasts 10 s. Each column acts alone.
fn smoke(units: Value, scripts: Value) -> ScenarioDefinition {
    let mut rules = common::game();
    rules["catalog"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "props": { "smoke": {
        "body": { "blocks": { "infantry": false, "vehicle": false }, "stops_rounds": false,
            "occludes": true, "weight_class": "immovable", "lifetime_s": 10 },
        "appearance": { "drawn_by": "smoke", "stand_in_tint": [0.5, 0.5, 0.5] },
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
                { "side": "blue", "kind": "test_rifle", "position": [84, 50], "engagement": "return_fire_only" },
                vehicle("blue", "test_tank", [72.0, 38.0]),
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
                { "side": "blue", "kind": "test_rifle", "position": [60, 50], "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_rifle", "position": [140, 50], "engagement": "return_fire_only" },
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
/// armoured, so rifle rounds get only partly through and every heavier row
/// gets fully through every face.
#[test]
fn a_jeep_sees_all_round_and_only_rifles_partly_hurt_it() {
    let r = rules();
    let arsenal = sim::weapons::Arsenal::new(&r);
    let rifle = &arsenal
        .weapons
        .iter()
        .find(|w| w.id == "rifle")
        .unwrap()
        .def;
    let a = common::hull("test_jeep").armor;
    let share = |w: &contract::weapons::WeaponDefinition, armor: f64| {
        sim::damage::pierce_share(w, 0, &r, armor)
    };
    for w in &arsenal.weapons {
        if w.def.penetration > rifle.penetration {
            let thickest = a.front.max(a.side).max(a.rear).max(a.roof);
            assert_eq!(share(&w.def, thickest), 1.0, "{}", w.id);
        }
    }
    let rifle_share = share(rifle, a.weakest());
    assert!(
        0.0 < rifle_share && rifle_share < 1.0,
        "rifle: {rifle_share}"
    );
    let range = r.catalog.by_id("test_jeep").sensors.ground_m;
    for dx in [-0.9 * range, 0.9 * range] {
        let setup = common::scenario(
            &json!({ "size": [1200, 200], "fog_cell_m": 8, "height_grid_m": 8, "slope_cutoff_deg": 35 }).to_string(),
            json!([
                vehicle("blue", "test_jeep", [600.0, 100.0]),
                { "side": "red", "kind": "test_rifle", "position": [600.0 + dx, 100], "engagement": "return_fire_only" },
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
            vehicle("blue", "test_jeep", [200.0, 100.0]),
            { "side": "red", "kind": "test_tank", "position": [320, 100], "yaw": std::f64::consts::PI },
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
    assert_eq!(
        b.world().types().id(wreck.kind),
        common::hull("test_jeep").wreck
    );
    assert!(common::hull("test_tank")
        .push_class
        .pushes(wreck.body.weight_class));
}

/// A new prop type is one catalog entry: a vehicle whose `wreck` names a
/// type added in a document of its own leaves that type, with its own body
/// row, and nothing in code knows its id.
#[test]
fn a_wreck_is_whatever_prop_type_its_vehicle_names() {
    let mut rules = common::game();
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
        "test_jeep",
        json!({ "body": { "hull": { "wreck": "burnt_out_jeep" } } }),
    );
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [400, 200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            vehicle("blue", "test_jeep", [200.0, 100.0]),
            { "side": "red", "kind": "test_tank", "position": [320, 100], "yaw": std::f64::consts::PI },
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

/// C44: a street lamp is a thin obstruction, never a wall or cover position.
#[test]
fn a_street_lamp_blocks_movers_without_hiding_or_sheltering_them() {
    let w = common::flat(
        [100.0, 100.0],
        r#","props":[{"kind":"lamp","center":[50,50],"yaw":0,"half_extents":[0.15,0.15,3]}]"#,
    );
    let p = w.prop(0).unwrap();
    assert!(p.blocks(MoverClass::Infantry) && p.blocks(MoverClass::Vehicle));
    assert!(w.sight_clear(v3(40.0, 50.0, 1.0), v3(60.0, 50.0, 1.0)));
    assert!(w.segment_clear(v3(40.0, 50.0, 1.0), v3(60.0, 50.0, 1.0)));
    let r = rules();
    let ground = GroundLayer::new(100.0, 100.0, &r.ground);
    assert_eq!(
        sim::cover::at(&w, &ground, &[], &r, v2(49.4, 50.0), v2(60.0, 50.0)),
        None
    );
}

/// Street bodies retain independent sight, fire and cover columns. Glass and
/// mesh must never become opaque bulletproof walls just because they block feet.
#[test]
fn street_body_rows_obey_their_physical_roles() {
    use contract::scenario::CoverTier::{Heavy, Light, Medium};
    let rows = [
        ("bench", false, false, Some(Light)),
        ("bollard", true, false, None),
        ("bins", false, false, Some(Light)),
        ("hydrant", true, false, None),
        ("utility_box", true, false, Some(Medium)),
        ("scooter", false, false, None),
        ("planter", true, false, Some(Medium)),
        ("parked_car", true, false, Some(Medium)),
        ("car_wreck", true, false, Some(Medium)),
        ("jersey_barrier", true, false, Some(Heavy)),
        ("bus_shelter", false, false, None),
        ("scaffold", false, false, None),
        ("heras_fence", false, false, None),
        ("skip_bin", true, false, Some(Medium)),
        ("pallet_stack", false, false, Some(Light)),
        ("site_cabin", true, true, Some(Heavy)),
        ("traffic_cone", false, false, None),
        ("road_barrier", false, false, None),
    ];
    let r = rules();
    let ground = GroundLayer::new(100.0, 100.0, &r.ground);
    for (id, stops, hides, cover) in rows {
        let w = common::flat(
            [100.0, 100.0],
            &format!(
                r#","props":[{{"kind":"{id}","center":[50,50],"yaw":0,"half_extents":[1,1,1]}}]"#
            ),
        );
        assert_eq!(
            w.segment_clear(v3(40.0, 50.0, 1.0), v3(60.0, 50.0, 1.0)),
            !stops,
            "{id}: rounds"
        );
        assert_eq!(
            w.sight_clear(v3(40.0, 50.0, 1.0), v3(60.0, 50.0, 1.0)),
            !hides,
            "{id}: sight"
        );
        assert_eq!(
            sim::cover::at(&w, &ground, &[], &r, v2(48.5, 50.0), v2(60.0, 50.0)),
            cover,
            "{id}: cover"
        );
        let p = w.prop(0).unwrap();
        assert_eq!(
            p.blocks(MoverClass::Infantry),
            id != "traffic_cone",
            "{id}: feet"
        );
        assert_eq!(
            p.blocks(MoverClass::Vehicle),
            id != "traffic_cone",
            "{id}: wheels"
        );
        assert!(
            !p.body.garrison,
            "{id}: street rows are not fighting-position buildings"
        );
    }
}
