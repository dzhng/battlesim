//! Smoke screens (`specs/smoke-screen`): a carrier pops a charge into a
//! stationary cloud. Every number is read from the carrier's own row.
use contract::catalog::SmokeCapability;
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

const TANK: &str = "test_smoke_tank";
const SQUAD: &str = "test_smoke_squad";

fn smoke(kind: &str) -> SmokeCapability {
    common::rules()
        .catalog
        .by_id(kind)
        .capabilities
        .smoke
        .clone()
        .unwrap_or_else(|| panic!("{kind} carries smoke"))
}

fn open_ground() -> String {
    json!({ "size": [600, 300], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
        .to_string()
}

fn unit(side: &str, kind: &str, at: [f64; 2]) -> Value {
    json!({ "side": side, "kind": kind, "position": at, "engagement": "return_fire_only" })
}

fn setup(units: Value) -> ScenarioDefinition {
    common::scenario_with(&open_ground(), units, json!([]), json!([]))
}

fn run(b: &mut Battle, ticks: u64) {
    for _ in 0..ticks {
        b.step();
    }
}

fn seconds(s: f64) -> u64 {
    (s * common::tick_hz() as f64).round() as u64
}

fn charges(b: &Battle, id: u32) -> u32 {
    b.unit(UnitId(id))
        .unwrap()
        .smoke
        .expect("a smoke carrier")
        .charges
}

fn clouds(b: &Battle) -> usize {
    b.world().smoke().live().count()
}

/// Send `order` for blue as command `seq`; it must be accepted.
fn send(b: &mut Battle, seq: u64, order: Order, queued: bool) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued,
    });
    assert_eq!(ack.error, None, "{ack:?}");
}

fn pop(units: &[u32]) -> Order {
    Order::PopSmoke {
        units: units.iter().map(|&id| UnitId(id)).collect(),
    }
}

fn move_to(id: u32, goal: [f64; 2]) -> Order {
    Order::Move {
        units: vec![UnitId(id)],
        gesture: u64::from(id) + 1,
        goal,
        route: RoutePolicy::Shortest,
        direction: MoveDirection::default(),
        facing: None,
    }
}

#[test]
fn a_moving_carrier_pops_smoke_and_keeps_its_move_and_its_queue() {
    let mut b = Battle::new(&setup(json!([unit("blue", TANK, [100.0, 150.0])])), 1);
    send(&mut b, 1, move_to(0, [400.0, 150.0]), false);
    send(&mut b, 2, move_to(0, [400.0, 250.0]), true);
    run(&mut b, seconds(3.0));
    let before = b.unit(UnitId(0)).unwrap().clone();
    assert!(before.moving(), "the tank is under way");
    send(&mut b, 3, pop(&[0]), false);
    b.step();
    let after = b.unit(UnitId(0)).unwrap();
    assert_eq!(charges(&b, 0), smoke(TANK).capacity - 1);
    assert!(after.moving(), "popping smoke doesn't stop it");
    let goals = |u: &sim::units::Unit| -> Vec<_> {
        u.orders
            .iter()
            .map(|o| o.movement().map(|m| m.destination))
            .collect()
    };
    assert_eq!(
        goals(after),
        goals(&before),
        "its move and its queue survive"
    );
    assert_eq!(goals(after).len(), 2);
    let (popped, _) = b.world().smoke().live().next().expect("a cloud");
    let cloud = &popped.cloud;
    let at = [
        cloud.centre[0] - before.position.x,
        cloud.centre[1] - before.position.y,
    ];
    assert!(
        (at[0] * at[0] + at[1] * at[1]).sqrt() < 1.0,
        "the cloud is centred where the tank popped it"
    );
}

#[test]
fn every_selected_carrier_pops_one_charge_and_the_rest_are_ignored_silently() {
    let mut b = Battle::new(
        &setup(json!([
            unit("blue", TANK, [100.0, 100.0]),
            unit("blue", SQUAD, [100.0, 200.0]),
            unit("blue", "test_tank", [200.0, 150.0]),
        ])),
        1,
    );
    send(&mut b, 1, pop(&[0, 1, 2]), false);
    b.step();
    assert_eq!(charges(&b, 0), smoke(TANK).capacity - 1);
    assert_eq!(charges(&b, 1), smoke(SQUAD).capacity - 1);
    assert_eq!(
        clouds(&b),
        2,
        "one cloud a carrier, none for the plain tank"
    );
    assert!(b.unit(UnitId(2)).unwrap().smoke.is_none());
    assert!(
        b.observe(Side::Blue).refusals.is_empty(),
        "a unit without smoke isn't refused"
    );
}

#[test]
fn a_carrier_out_of_charges_pops_nothing() {
    let mut b = Battle::new(&setup(json!([unit("blue", SQUAD, [100.0, 150.0])])), 1);
    let capacity = smoke(SQUAD).capacity;
    for seq in 1..=u64::from(capacity) + 1 {
        send(&mut b, seq, pop(&[0]), false);
        b.step();
    }
    assert_eq!(charges(&b, 0), 0);
    assert_eq!(clouds(&b), capacity as usize, "the extra pop made no cloud");
}

#[test]
fn a_cloud_lasts_its_life_and_then_is_gone() {
    let mut b = Battle::new(&setup(json!([unit("blue", SQUAD, [100.0, 150.0])])), 1);
    send(&mut b, 1, pop(&[0]), false);
    b.step();
    let life = seconds(smoke(SQUAD).cloud.life_s);
    run(&mut b, life - 1);
    assert_eq!(clouds(&b), 1, "still there on its last tick");
    b.step();
    assert_eq!(clouds(&b), 0, "gone once its life is over");
}

#[test]
fn a_pop_changes_no_obstacle_or_navigation_revision() {
    let mut b = Battle::new(
        &setup(json!([
            unit("blue", TANK, [100.0, 150.0]),
            unit("red", "test_tank", [500.0, 150.0]),
        ])),
        1,
    );
    run(&mut b, 5);
    let revisions = |b: &Battle| {
        [
            b.world().obstacle_revision(),
            b.navigation_revision(Side::Blue),
            b.navigation_revision(Side::Red),
        ]
    };
    let before = revisions(&b);
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, seconds(smoke(TANK).cloud.life_s) + 2);
    assert_eq!(revisions(&b), before, "through the pop and the cloud's end");
}

/// A truck in reach; the carrier stands still and fires at nothing.
fn resupplied(carrier: Value) -> Battle {
    Battle::new(
        &setup(json!([
            carrier,
            { "side": "blue", "kind": "test_supply", "position": [130, 150] },
        ])),
        1,
    )
}

#[test]
fn a_truck_restores_one_smoke_charge_a_delivery_at_its_price() {
    let mut b = resupplied(unit("blue", TANK, [100.0, 150.0]));
    let cap = smoke(TANK);
    for seq in 1..=u64::from(cap.capacity) {
        send(&mut b, seq, pop(&[0]), false);
        b.step();
    }
    assert_eq!(charges(&b, 0), 0);
    let stock = |b: &Battle| b.unit(UnitId(1)).unwrap().stock.unwrap();
    let mut deliveries = Vec::new();
    let mut last = (0, stock(&b));
    for tick in 0..seconds(60.0) {
        b.step();
        let now = (charges(&b, 0), stock(&b));
        if now != last {
            assert_eq!(now.0, last.0 + 1, "one charge a delivery");
            assert_eq!(last.1 - now.1, cap.stock_per_charge, "paid at its price");
            deliveries.push(tick);
            last = now;
        }
    }
    assert_eq!(
        deliveries.len(),
        cap.capacity as usize,
        "refilled to capacity, no more"
    );
    let apart = deliveries.windows(2).map(|w| w[1] - w[0]).min();
    assert!(
        apart.is_none_or(|t| t >= seconds(cap.service_s)),
        "each charge takes its service time"
    );
}

#[test]
fn replacement_soldiers_are_served_before_smoke() {
    let mut b = resupplied(json!({
        "side": "blue", "kind": SQUAD, "position": [100, 150],
        "engagement": "return_fire_only", "condition": { "casualties": 1 }
    }));
    send(&mut b, 1, pop(&[0]), false);
    let living = |b: &Battle| {
        b.unit(UnitId(0))
            .unwrap()
            .members
            .iter()
            .filter(|s| s.alive())
            .count()
    };
    let short = living(&b);
    let (mut soldier, mut charge) = (None, None);
    for tick in 0..seconds(60.0) {
        b.step();
        if soldier.is_none() && living(&b) > short {
            soldier = Some(tick);
        }
        if charge.is_none() && charges(&b, 0) > 0 {
            charge = Some(tick);
        }
    }
    let (soldier, charge) = (
        soldier.expect("a soldier joins"),
        charge.expect("smoke returns"),
    );
    assert!(
        soldier < charge,
        "the soldier at {soldier}, the smoke at {charge}"
    );
}

#[test]
fn a_battle_with_pops_replays_to_the_same_digests() {
    let units = json!([
        unit("blue", TANK, [100.0, 100.0]),
        unit("blue", SQUAD, [100.0, 200.0]),
        unit("red", "test_tank", [500.0, 150.0]),
    ]);
    let setup = setup(units);
    let mut live = Battle::new(&setup, 7);
    let mut digests = Vec::new();
    for tick in 0..seconds(45.0) {
        match tick {
            3 => send(&mut live, 1, pop(&[0, 1]), false),
            90 => send(&mut live, 2, pop(&[0]), false),
            _ => {}
        }
        live.step();
        digests.push(live.digest());
    }
    let mut quiet = Battle::new(&setup, 7);
    run(&mut quiet, 10);
    assert_ne!(quiet.digest(), digests[9], "a pop is battle state");
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    for (tick, expected) in digests.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.digest(),
            *expected,
            "first mismatch at tick {}",
            tick + 1
        );
    }
}

mod players;
mod publication;
mod sight;
