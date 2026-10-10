//! Soldiers are bodies: each has his own position on the ground
//! under him and collides with solid props on his own; the squad's position
//! is where its living soldiers stand; every move ends in a fresh seeded
//! arrangement, never a formation.
use contract::ids::UnitId;
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::V2;

use crate::common;

fn setup(map: Value, units: Value, scripts: Value) -> ScenarioDefinition {
    let mut m =
        json!({ "size": [120, 80], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 });
    for (k, v) in map.as_object().unwrap() {
        m[k] = v.clone();
    }
    serde_json::from_value(json!({
        "map": m, "rules": common::scenario_rules(), "units": units, "events": [], "scripts": scripts,
    }))
    .unwrap()
}

fn rifle(at: [f64; 2]) -> Value {
    json!({ "side": "blue", "kind": "test_rifle", "position": at, "engagement": "return_fire_only" })
}

fn go(tick: u64, unit: u32, gesture: u64, goal: [f64; 2]) -> Value {
    json!({ "tick": tick, "side": "blue", "order": {
        "kind": "move", "units": [unit], "gesture": gesture, "goal": goal, "route": "shortest" } })
}

fn soldiers(b: &Battle, unit: u32) -> Vec<V2> {
    b.unit(UnitId(unit))
        .unwrap()
        .member_positions()
        .map(|p| p.xy())
        .collect()
}

#[test]
fn a_corridor_member_walks_round_idle_soldiers_while_his_squadmate_advances() {
    // Two scouts leave less than one soldier's diameter between them. One
    // walker starts against their discs; his squadmate has a clear lane.
    // The advancing man must not hide the other's need to go round them.
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({"body":{"squad":{"slots":["test_rifleman","test_rifleman"]}}}),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_recon",
        json!({"body":{"squad":{"slots":["test_scout"]}}}),
    );
    rules["physics"]["soldier_radius_m"] = json!(0.3);
    let mut s: ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,3000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":rules,
        "units":[
            rifle([500.0,100.0]),
            {"side":"blue","kind":"test_recon","position":[500.4159144498272,100.43245389398865],"engagement":"return_fire_only"},
            {"side":"blue","kind":"test_recon","position":[499.3909583746914,100.04399329150203],"engagement":"return_fire_only"}
        ],"scripts":[],"events":[]
    }))
    .unwrap();
    // Freeze the contact geometry through the public scenario input while
    // leaving the seeded arrangement of the two walkers to its owner.
    let start = sim::math::v2(500.0, 100.0);
    let initial = Battle::new(&s, 1);
    let offset = start - soldiers(&initial, 0)[0];
    s.units[0].position = [500.0 + offset.x, 100.0 + offset.y];
    let mut b = Battle::new(&s, 1);
    let before = soldiers(&b, 0);
    let scouts = [soldiers(&b, 1)[0], soldiers(&b, 2)[0]];
    let ack = b.accept(contract::command::CommandEnvelope {
        side: contract::ids::Side::Blue,
        seq: 1,
        queued: false,
        order: contract::command::Order::Move {
            units: vec![UnitId(0)],
            gesture: 2,
            goal: [100.0, 2500.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    });
    assert_eq!(ack.error, None);
    assert!(ack.placement.unwrap().destinations[0].placed);
    for _ in 0..30 * b.rules().tick_hz {
        b.step();
        let bodies: Vec<_> = (0..3).flat_map(|id| soldiers(&b, id)).collect();
        for (i, p) in bodies.iter().enumerate() {
            for q in &bodies[i + 1..] {
                assert!((*p - *q).length() >= 0.6 - 1e-9, "tick {}", b.tick());
            }
        }
    }
    assert_eq!([soldiers(&b, 1)[0], soldiers(&b, 2)[0]], scouts);
    let after = soldiers(&b, 0);
    assert!(
        (after[1] - before[1]).length() > 20.0,
        "the clear walker advances"
    );
    assert!(
        (after[0] - start).length() > 20.0,
        "the other walker remains at the scouts while his squadmate advances: {:?}",
        after[0]
    );
}

fn crate_at(center: [f64; 2]) -> Value {
    json!({ "kind": "crate", "center": center, "yaw": 0.4, "half_extents": [0.8, 0.8, 0.6] })
}

/// Distance from `p` to the prop's footprint (0 inside).
fn clearance(prop: &sim::world::Prop, p: V2) -> f64 {
    let d = prop.footprint().to_local(p);
    let (x, y) = (
        (d.x.abs() - prop.half.x).max(0.0),
        (d.y.abs() - prop.half.y).max(0.0),
    );
    x.hypot(y)
}

#[test]
fn no_soldier_ever_stands_inside_a_prop_on_the_way_through_a_door_into_crates() {
    // A wall with a 5 m door, and a crate yard around the goal beyond it.
    let props = json!([
        { "kind": "wall", "center": [60.0, 18.75], "yaw": 0.0, "half_extents": [0.5, 18.75, 1.5] },
        { "kind": "wall", "center": [60.0, 61.25], "yaw": 0.0, "half_extents": [0.5, 18.75, 1.5] },
        crate_at([90.0, 40.0]), crate_at([93.0, 43.0]), crate_at([88.0, 36.0]),
        crate_at([95.0, 38.0]), crate_at([91.0, 46.0]),
    ]);
    let s = setup(
        json!({ "props": props }),
        json!([rifle([20.0, 50.0])]),
        json!([go(1, 0, 1, [92.0, 41.0])]),
    );
    let radius = common::physics("soldier_radius_m");
    let mut b = Battle::new(&s, 3);
    for _ in 0..60 * 30 {
        b.step();
        for p in soldiers(&b, 0) {
            for prop in b.world().props() {
                assert!(
                    clearance(prop, p) >= radius - 1e-6,
                    "tick {}: a soldier at {:.2},{:.2} stands in {:?} {}",
                    b.tick(),
                    p.x,
                    p.y,
                    prop.kind,
                    prop.id
                );
            }
        }
    }
    assert!(
        b.unit(UnitId(0)).unwrap().orders.is_empty(),
        "the squad arrived"
    );
}

/// Each soldier's place relative to the squad's middle.
fn shape(b: &Battle, unit: u32) -> Vec<V2> {
    let c = b.unit(UnitId(unit)).unwrap().position.xy();
    soldiers(b, unit).into_iter().map(|p| p - c).collect()
}

fn walk_twice(seed: u64) -> (Vec<V2>, Vec<V2>, Battle) {
    // Out to a point, away, and on to a new point by a third order.
    let s = setup(
        json!({}),
        json!([rifle([20.0, 40.0])]),
        json!([
            go(1, 0, 1, [60.0, 40.0]),
            go(600, 0, 2, [30.0, 40.0]),
            go(900, 0, 3, [60.0, 50.0]),
        ]),
    );
    let mut b = Battle::new(&s, seed);
    for _ in 0..599 {
        b.step();
    }
    let first = shape(&b, 0);
    for _ in 599..1500 {
        b.step();
    }
    (first, shape(&b, 0), b)
}

#[test]
fn every_new_goal_ends_in_a_fresh_seeded_arrangement() {
    let (first, second, b) = walk_twice(1);
    let u = b.unit(UnitId(0)).unwrap();
    assert!(u.orders.is_empty(), "every walk arrived");
    // Arrived around the ordered point, spread out and spaced (D1, Q7).
    let im = &common::game()["infantry_movement"];
    let spacing = im["spacing_m"].as_f64().unwrap();
    let spread = im["spread_m"].as_f64().unwrap();
    assert!((u.position.xy() - sim::math::v2(60.0, 50.0)).length() < 1.0);
    for arrangement in [&first, &second] {
        for (i, p) in arrangement.iter().enumerate() {
            assert!(p.length() <= spread, "a soldier {:.1} m out", p.length());
            for q in &arrangement[i + 1..] {
                assert!((*p - *q).length() >= spacing - 1e-9);
            }
        }
    }
    // Two goals end in two different arrangements.
    let moved = first
        .iter()
        .zip(&second)
        .filter(|(a, b)| (**a - **b).length() > 0.5)
        .count();
    assert!(
        moved >= first.len() / 2,
        "only {moved} soldiers stand elsewhere"
    );
    // The same seed replays the same arrangements; another seed differs.
    let (again, again2, _) = walk_twice(1);
    assert_eq!((first.clone(), second), (again, again2));
    let (other, _, _) = walk_twice(2);
    assert_ne!(first, other);
}

#[test]
fn the_squad_stands_where_its_living_soldiers_stand() {
    let s = setup(
        json!({ "relief": [{ "kind": "ridge", "center": [60, 40], "peak_m": 8, "radius_m": 30 }] }),
        json!([{ "side": "blue", "kind": "test_rifle", "position": [10, 36], "condition": { "casualties": 3 } }]),
        json!([go(1, 0, 1, [110.0, 44.0])]),
    );
    let mut b = Battle::new(&s, 1);
    for _ in 0..40 * 30 {
        b.step();
        let u = b.unit(UnitId(0)).unwrap();
        let living: Vec<_> = u.member_positions().collect();
        assert_eq!(living.len(), 5);
        let n = living.len() as f64;
        let mean = living.iter().fold(sim::math::V3::default(), |a, &p| a + p) * (1.0 / n);
        assert!((u.position - mean).length() < 1e-9, "tick {}", b.tick());
    }
}
