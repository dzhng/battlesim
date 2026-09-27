//! Order markers and the Space overlay (slice 35, D2, D2+, Q9, Q31): the
//! publication carries each own soldier's resolved spot and cover and each
//! unit's final facing, a right-drag's facing is kept at the end of the
//! move, and nothing of the enemy's plan reaches the other side.
use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::wrap_angle;
use sim::publication::Publisher;

fn rules() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/village.json")).unwrap()
}

fn setup(size: [f64; 2], props: Value, units: Value, scripts: Value) -> ScenarioDefinition {
    serde_json::from_value(json!({
        "map": { "size": size, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props },
        "rules": rules(),
        "units": units,
        "events": [],
        "scripts": scripts,
    }))
    .unwrap()
}

fn mv(tick: u64, side: &str, unit: u32, goal: [f64; 2], extra: Value) -> Value {
    let mut order = json!({ "kind": "move", "units": [unit], "gesture": tick,
        "goal": goal, "route": "shortest" });
    order
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    json!({ "tick": tick, "side": side, "order": order })
}

fn own(b: &Battle, id: u32) -> OwnUnit {
    b.observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .clone()
}

fn run_until_idle(b: &mut Battle, id: u32, seconds: u64) {
    for _ in 0..seconds * 30 {
        b.step();
        if b.tick() > 3 && b.unit(UnitId(id)).unwrap().state == MoveState::Idle {
            return;
        }
    }
    panic!("unit {id} never arrived");
}

#[test]
fn a_squads_published_spots_and_cover_are_its_soldiers_resolved_ones() {
    // A long wall between the squad's goal and a red squad far east: the
    // order resolves spots behind it (D4), and the publication shows them.
    let wall = json!({ "kind": "wall", "center": [100.0, 60.0], "yaw": 0,
        "half_extents": [0.4, 14.0, 0.6] });
    let s = setup(
        [260.0, 120.0],
        json!([wall]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [40, 60], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [200, 60], "engagement": "return_fire_only" },
        ]),
        json!([mv(1, "blue", 0, [97.0, 60.0], json!({}))]),
    );
    let mut b = Battle::new(&s, 4);
    b.step();
    b.step();
    let check = |b: &Battle| {
        let published = own(b, 0);
        let unit = b.unit(UnitId(0)).unwrap();
        let living: Vec<_> = unit.members.iter().filter(|s| s.alive()).collect();
        assert_eq!(published.member_orders.len(), living.len());
        for (m, s) in published.member_orders.iter().zip(&living) {
            let spot = s.spot.or(s.post).unwrap_or(s.position.xy());
            assert_eq!(m.spot, [spot.x, spot.y], "soldier {}", s.id);
            assert_eq!(m.cover_there, s.cover, "soldier {}", s.id);
        }
        published
    };
    let moving = check(&b);
    assert!(
        moving.member_orders.iter().any(|m| m.cover_there.is_some()),
        "some spots lie behind the wall: {:?}",
        moving.member_orders
    );
    assert!(
        moving.member_orders.iter().all(|m| m.cover_now.is_none()),
        "nobody has cover in the open at the start"
    );
    run_until_idle(&mut b, 0, 60);
    let there = check(&b);
    let covered_now = there
        .member_orders
        .iter()
        .filter(|m| m.cover_now.is_some())
        .count();
    assert!(
        covered_now > 0,
        "arrived behind the wall: {:?}",
        there.member_orders
    );
}

#[test]
fn a_right_drag_facing_is_published_and_kept_by_a_squad_and_a_tank_but_not_by_wheels() {
    let north = std::f64::consts::FRAC_PI_2;
    let s = setup(
        [200.0, 200.0],
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": [40, 40], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "tank", "position": [40, 100], "yaw": 0, "engagement": "return_fire_only" },
            { "side": "blue", "kind": "jeep", "position": [40, 160], "yaw": 0, "engagement": "return_fire_only" },
        ]),
        json!([
            mv(1, "blue", 0, [100.0, 40.0], json!({ "facing": north })),
            mv(1, "blue", 1, [100.0, 100.0], json!({ "facing": north })),
            mv(1, "blue", 2, [100.0, 160.0], json!({ "facing": north })),
        ]),
    );
    let mut b = Battle::new(&s, 2);
    b.step();
    b.step();
    assert!((own(&b, 0).final_facing - north).abs() < 1e-9, "squad");
    assert!((own(&b, 1).final_facing - north).abs() < 1e-9, "tank");
    // Wheels never pivot: the jeep's marker shows the way it will come in.
    assert!(wrap_angle(own(&b, 2).final_facing).abs() < 0.05, "jeep");
    for _ in 0..40 * 30 {
        b.step();
    }
    let yaw = |id: u32| b.unit(UnitId(id)).unwrap().yaw;
    assert!(
        wrap_angle(yaw(0) - north).abs() < 1e-9,
        "the squad faces the drag"
    );
    assert!(
        wrap_angle(yaw(1) - north).abs() < 1e-6,
        "the tank pivoted to it"
    );
    assert!(wrap_angle(yaw(2)).abs() < 0.1, "the jeep kept its heading");
    for id in 0..3 {
        assert!(
            (own(&b, id).final_facing - yaw(id)).abs() < 1e-9,
            "at rest the marker is the unit's own facing"
        );
    }
}

#[test]
fn a_reversing_units_final_marker_shows_its_held_facing() {
    let s = setup(
        [200.0, 120.0],
        json!([]),
        json!([{ "side": "blue", "kind": "tank", "position": [120, 60], "yaw": 0,
            "engagement": "return_fire_only" }]),
        json!([mv(
            1,
            "blue",
            0,
            [60.0, 60.0],
            json!({ "direction": "reverse" })
        )]),
    );
    let mut b = Battle::new(&s, 1);
    b.step();
    b.step();
    let u = own(&b, 0);
    assert_eq!(u.direction, Some(contract::command::MoveDirection::Reverse));
    assert!(
        wrap_angle(u.final_facing).abs() < 1e-9,
        "held facing east: {}",
        u.final_facing
    );
}

/// Every record blue is published over `ticks`, bit for bit.
fn blue_records(s: &ScenarioDefinition, ticks: usize) -> Vec<Vec<u32>> {
    let mut b = Battle::new(s, 5);
    let mut p = Publisher::new();
    (0..ticks)
        .map(|_| {
            b.step();
            p.publish(&b, Side::Blue)
                .iter()
                .map(|v| v.to_bits())
                .collect()
        })
        .collect()
}

#[test]
fn nothing_of_the_enemys_plan_reaches_the_other_side() {
    // The same battle, except red is ordered somewhere else, with a facing,
    // while blue cannot see it: blue's publications stay bit-identical.
    let units = json!([
        { "side": "blue", "kind": "rifle", "position": [40, 60], "engagement": "return_fire_only" },
        { "side": "red", "kind": "tank", "position": [1150, 60], "engagement": "return_fire_only" },
    ]);
    let blue = mv(1, "blue", 0, [70.0, 60.0], json!({}));
    let plan = |goal: [f64; 2], extra: Value| {
        setup(
            [1200.0, 120.0],
            json!([]),
            units.clone(),
            json!([blue.clone(), mv(1, "red", 1, goal, extra)]),
        )
    };
    let a = blue_records(&plan([1150.0, 20.0], json!({})), 90);
    let b = blue_records(
        &plan(
            [1100.0, 100.0],
            json!({ "facing": 2.0, "direction": "reverse" }),
        ),
        90,
    );
    assert_eq!(a, b, "blue's view never changes with red's orders");
}
