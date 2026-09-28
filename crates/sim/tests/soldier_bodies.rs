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
    let mut m = json!({ "size": [120, 80], "height_grid_m": 4, "slope_cutoff_deg": 35 });
    for (k, v) in map.as_object().unwrap() {
        m[k] = v.clone();
    }
    serde_json::from_value(json!({
        "map": m, "rules": common::scenario_rules(), "units": units, "events": [], "scripts": scripts,
    }))
    .unwrap()
}

fn rifle(at: [f64; 2]) -> Value {
    json!({ "side": "blue", "kind": "rifle", "position": at, "engagement": "return_fire_only" })
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
fn every_soldier_stands_on_the_ground_under_him_while_crossing_a_ridge() {
    let s = setup(
        json!({ "relief": [{ "kind": "ridge", "center": [60, 40], "peak_m": 8, "radius_m": 30 }] }),
        json!([rifle([10.0, 36.0])]),
        json!([go(1, 0, 1, [110.0, 44.0])]),
    );
    let mut b = Battle::new(&s, 1);
    let mut checked = 0;
    for _ in 0..45 * 30 {
        b.step();
        for p in b.unit(UnitId(0)).unwrap().member_positions() {
            let ground = b.world().height_at(p.x, p.y).unwrap();
            assert!(
                (p.z - ground).abs() < 1e-6,
                "tick {}: a soldier at {:.2},{:.2} stands at {:.3}, the ground at {:.3}",
                b.tick(),
                p.x,
                p.y,
                p.z,
                ground
            );
            checked += 1;
        }
    }
    assert!(checked > 0);
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
    // Out to a point, away, and back to it by a third order: two arrivals there.
    let s = setup(
        json!({}),
        json!([rifle([20.0, 40.0])]),
        json!([
            go(1, 0, 1, [60.0, 40.0]),
            go(600, 0, 2, [30.0, 40.0]),
            go(900, 0, 3, [60.0, 40.0]),
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
fn every_move_ends_in_a_fresh_seeded_arrangement() {
    let (first, second, b) = walk_twice(1);
    let u = b.unit(UnitId(0)).unwrap();
    assert!(u.orders.is_empty(), "every walk arrived");
    // Arrived around the ordered point, spread out and spaced (D1, Q7).
    let im = &common::village()["infantry_movement"];
    let spacing = im["spacing_m"].as_f64().unwrap();
    let spread = im["spread_m"].as_f64().unwrap();
    assert!((u.position.xy() - sim::math::v2(60.0, 40.0)).length() < 1.0);
    for arrangement in [&first, &second] {
        for (i, p) in arrangement.iter().enumerate() {
            assert!(p.length() <= spread, "a soldier {:.1} m out", p.length());
            for q in &arrangement[i + 1..] {
                assert!((*p - *q).length() >= spacing - 1e-9);
            }
        }
    }
    // The same order twice ends in two different arrangements.
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
        json!([{ "side": "blue", "kind": "rifle", "position": [10, 36], "condition": { "casualties": 3 } }]),
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
