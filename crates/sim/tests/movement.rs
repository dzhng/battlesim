//! Movement contracts at the authority: routes, groups, traffic, knowledge.
use contract::command::{CommandEnvelope, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use sim::battle::Battle;

use crate::common;

fn scenario(units: serde_json::Value, events: serde_json::Value) -> ScenarioDefinition {
    common::scenario(common::GEOMETRY_LAB, units, events)
}

struct Orders {
    seq: u64,
}
impl Orders {
    fn send(&mut self, b: &mut Battle, order: Order, queued: bool) {
        self.seq += 1;
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: self.seq,
            order,
            queued,
        });
        assert_eq!(ack.error, None);
    }
    fn go(
        &mut self,
        b: &mut Battle,
        units: &[u32],
        goal: [f64; 2],
        gesture: u64,
        route: RoutePolicy,
        queued: bool,
    ) {
        let units = units.iter().map(|&u| UnitId(u)).collect();
        self.send(
            b,
            Order::Move {
                units,
                gesture,
                goal,
                route,
                direction: contract::command::MoveDirection::Forward,
                facing: None,
            },
            queued,
        );
    }
}

fn own(b: &Battle, id: u32) -> OwnUnit {
    b.observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .clone()
}

fn xy(u: &OwnUnit) -> [f64; 2] {
    [u.position[0], u.position[1]]
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Step until every listed unit is idle or `limit` ticks pass; checks `each` every tick.
fn run(b: &mut Battle, ids: &[u32], limit: u32, mut each: impl FnMut(&Battle)) -> u32 {
    for t in 0..limit {
        b.step();
        each(b);
        if ids.iter().all(|&id| own(b, id).state == MoveState::Idle) {
            return t;
        }
    }
    limit
}

#[test]
fn a_tank_routes_around_walls_and_arrives() {
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "tank", "position": [60, 40] }]),
            serde_json::json!([]),
        ),
        1,
    );
    let mut o = Orders { seq: 0 };
    o.go(&mut b, &[0], [60.0, 110.0], 1, RoutePolicy::Shortest, false);
    let ticks = run(&mut b, &[0], 3000, |b| {
        let p = xy(&own(b, 0));
        for prop in b.world().props().filter(|p| p.blocks(MoverClass::Vehicle)) {
            assert!(
                !prop.footprint().contains(sim::math::v2(p[0], p[1]), 0.0),
                "inside a prop at {p:?}"
            );
        }
    });
    assert!(ticks < 3000);
    assert!(dist(xy(&own(&b, 0)), [60.0, 110.0]) < 1.0);
    assert!(
        b.route_searches(Side::Blue) <= 3,
        "searches {}",
        b.route_searches(Side::Blue)
    );
}

#[test]
fn a_group_keeps_its_arrangement_and_each_unit_its_own_speed() {
    let units = serde_json::json!([
        { "side": "blue", "kind": "tank", "position": [30, 150] },
        { "side": "blue", "kind": "rifle", "position": [30, 170] },
        { "side": "blue", "kind": "supply", "position": [30, 130] },
    ]);
    let mut b = Battle::new(&scenario(units, serde_json::json!([])), 1);
    let mut o = Orders { seq: 0 };
    o.go(
        &mut b,
        &[0, 1, 2],
        [150.0, 150.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    b.step();
    let goals: Vec<[f64; 2]> = (0..3).map(|i| own(&b, i).goal.unwrap()).collect();
    // Relative arrangement preserved: rifle north of the tank, supply south.
    assert!(
        (goals[1][1] - goals[0][1] - 20.0).abs() < 2.5
            && (goals[0][1] - goals[2][1] - 20.0).abs() < 2.5,
        "{goals:?}"
    );
    let mut arrived = [None; 3];
    run(&mut b, &[0, 1, 2], 3000, |b| {
        for (i, a) in arrived.iter_mut().enumerate() {
            if a.is_none() && own(b, i as u32).state == MoveState::Idle {
                *a = Some(b.tick());
            }
        }
    });
    let [tank, rifle, supply] = arrived.map(Option::unwrap);
    assert!(
        tank < rifle && supply < rifle,
        "infantry is slowest: {arrived:?}"
    );
    for i in 0..3 {
        assert!(
            dist(xy(&own(&b, i)), goals[i as usize]) < 1.5,
            "unit {i} at its own slot"
        );
    }
}

#[test]
fn a_new_obstacle_is_learned_on_contact_and_routed_around() {
    // A wall appears across the tank's straight path at tick 5; red is far away.
    let units = serde_json::json!([
        { "side": "blue", "kind": "tank", "position": [230, 120] },
        { "side": "red", "kind": "tank", "position": [370, 280] },
    ]);
    let events = serde_json::json!([{ "tick": 5, "add_prop":
        { "kind": "wall", "center": [260, 120], "yaw": 0, "half_extents": [0.5, 15, 2] } }]);
    let mut b = Battle::new(&scenario(units, events), 1);
    let mut o = Orders { seq: 0 };
    o.go(
        &mut b,
        &[0],
        [300.0, 120.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    b.step();
    let first = own(&b, 0).route.clone();
    assert_eq!(first.len(), 1, "unknown wall: straight route");
    let wall = sim::math::v2(260.0, 120.0);
    let ticks = run(&mut b, &[0], 2000, |b| {
        let p = xy(&own(b, 0));
        assert!(
            (p[0] - wall.x).abs() > 0.5 || (p[1] - wall.y).abs() > 15.0,
            "drove through the wall at {p:?}"
        );
    });
    assert!(ticks < 2000 && dist(xy(&own(&b, 0)), [300.0, 120.0]) < 1.0);
    assert!(
        b.route_searches(Side::Blue) <= 4,
        "searches {}",
        b.route_searches(Side::Blue)
    );
    assert_eq!(b.route_searches(Side::Red), 0, "red never planned");
}

#[test]
fn an_unreachable_destination_is_kept_and_not_searched_every_tick() {
    // The building's interior is solid.
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "tank", "position": [300, 150] }]),
            serde_json::json!([]),
        ),
        1,
    );
    let mut o = Orders { seq: 0 };
    // The 45° plateau top is flat but walled by slopes past the cutoff.
    o.go(&mut b, &[0], [330.0, 50.0], 1, RoutePolicy::Shortest, false);
    for _ in 0..300 {
        b.step();
    }
    let u = own(&b, 0);
    assert_eq!(u.state, MoveState::RouteBlocked);
    assert!(u.goal.is_some(), "destination retained");
    assert_eq!(b.route_searches(Side::Blue), 1);
    // A new order replaces it and plans again.
    o.go(
        &mut b,
        &[0],
        [320.0, 150.0],
        2,
        RoutePolicy::Shortest,
        false,
    );
    b.step();
    assert_eq!(own(&b, 0).state, MoveState::Moving);
}

#[test]
fn double_click_upgrades_only_its_own_gesture_even_after_it_applied() {
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "tank", "position": [30, 150] }]),
            serde_json::json!([]),
        ),
        1,
    );
    let mut o = Orders { seq: 0 };
    o.go(
        &mut b,
        &[0],
        [150.0, 150.0],
        7,
        RoutePolicy::Shortest,
        false,
    );
    o.go(&mut b, &[0], [150.0, 100.0], 8, RoutePolicy::Shortest, true);
    o.go(&mut b, &[0], [150.0, 60.0], 7, RoutePolicy::Shortest, true);
    for _ in 0..5 {
        b.step();
    }
    o.send(
        &mut b,
        Order::UpgradeMove {
            gesture: 7,
            route: RoutePolicy::Fastest,
        },
        false,
    );
    b.step();
    let u = own(&b, 0);
    assert_eq!(u.policy, Some(RoutePolicy::Fastest));
    assert_eq!(
        u.queue,
        vec![[150.0, 100.0], [150.0, 60.0]],
        "no waypoint added or removed"
    );
    // Upgrading a finished gesture is an acknowledged no-op.
    run(&mut b, &[0], 6000, |_| {});
    let searches = b.route_searches(Side::Blue);
    o.send(
        &mut b,
        Order::UpgradeMove {
            gesture: 7,
            route: RoutePolicy::Fastest,
        },
        false,
    );
    b.step();
    assert_eq!(own(&b, 0).state, MoveState::Idle);
    assert_eq!(b.route_searches(Side::Blue), searches);
}

#[test]
fn head_on_vehicles_pass_without_overlapping() {
    let units = serde_json::json!([
        { "side": "blue", "kind": "tank", "position": [40, 25], "yaw": 0 },
        { "side": "blue", "kind": "tank", "position": [100, 25], "yaw": std::f64::consts::PI },
    ]);
    let mut b = Battle::new(&scenario(units, serde_json::json!([])), 1);
    let mut o = Orders { seq: 0 };
    o.go(&mut b, &[0], [100.0, 25.0], 1, RoutePolicy::Shortest, false);
    o.go(&mut b, &[1], [40.0, 25.0], 2, RoutePolicy::Shortest, false);
    let end = run(&mut b, &[0, 1], 3000, |b| {
        let (a, c) = (own(b, 0), own(b, 1));
        assert!(
            dist(xy(&a), xy(&c)) > 3.5,
            "hulls overlapping at tick {}",
            b.tick()
        );
        for u in [&a, &c] {
            if u.state == MoveState::Waiting {
                assert!(u.blocker.is_some(), "a wait names its blocker");
            }
        }
    });
    assert!(
        end < 3000,
        "the lower-priority tank detours and both arrive"
    );
    assert!(
        dist(xy(&own(&b, 0)), [100.0, 25.0]) < 1.0 && dist(xy(&own(&b, 1)), [40.0, 25.0]) < 1.0
    );
    assert!(
        b.route_searches(Side::Blue) < 12,
        "bounded replanning: {}",
        b.route_searches(Side::Blue)
    );
}

#[test]
fn a_squad_walks_around_a_parked_tank() {
    let units = serde_json::json!([
        { "side": "blue", "kind": "tank", "position": [100, 25], "yaw": std::f64::consts::FRAC_PI_2 },
        { "side": "blue", "kind": "rifle", "position": [60, 25] },
    ]);
    let mut b = Battle::new(&scenario(units, serde_json::json!([])), 1);
    let mut o = Orders { seq: 0 };
    o.go(&mut b, &[1], [140.0, 25.0], 1, RoutePolicy::Shortest, false);
    let hull = b.unit(UnitId(0)).unwrap().hull_box().unwrap();
    let radius = common::physics("soldier_radius_m");
    let end = run(&mut b, &[1], 3000, |b| {
        // No soldier's body ever enters the tank's hull.
        for p in b.unit(UnitId(1)).unwrap().member_positions() {
            assert!(!hull.contains(p.xy(), radius), "through the tank at {p:?}");
        }
    });
    assert!(end < 3000, "the squad got past");
    assert_eq!(own(&b, 1).members.len(), 8);
}

#[test]
fn idle_units_never_search() {
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "rifle", "position": [60, 150] }]),
            serde_json::json!([]),
        ),
        1,
    );
    for _ in 0..600 {
        b.step();
    }
    assert_eq!(b.route_searches(Side::Blue), 0);
}

/// A 120 × 80 m flat map with these props, one blue rifle squad at `from`
/// sent to `goal` on tick 1.
fn one_squad(props: serde_json::Value, from: [f64; 2], goal: [f64; 2]) -> Battle {
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": { "size": [120, 80], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props },
        "rules": common::village(),
        "units": [{ "side": "blue", "kind": "rifle", "position": from, "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order": {
            "kind": "move", "units": [0], "gesture": 1, "goal": goal, "route": "shortest" } }],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

/// Each order costs the squad's corridor and at most one route of each
/// soldier's own (the final stretch, or back to the corridor): route
/// searches per order stay within 1 + soldiers, never one per tick.
#[test]
fn an_order_searches_at_most_once_for_the_squad_and_once_per_soldier() {
    let wall = |y: f64, half: f64| serde_json::json!({ "kind": "wall", "center": [60, y], "yaw": 0, "half_extents": [0.5, half, 1.5] });
    for (props, what) in [
        (serde_json::json!([]), "open ground"),
        (
            serde_json::json!([wall(18.75, 18.75), wall(61.25, 18.75)]),
            "a 5 m gap",
        ),
        (
            serde_json::json!([wall(19.625, 19.625), wall(60.375, 19.625)]),
            "a 1.5 m gap",
        ),
    ] {
        let mut b = one_squad(props, [20.0, 40.0], [100.0, 40.0]);
        let soldiers = b.unit(UnitId(0)).unwrap().members.len() as u64;
        let mut ticks = 0;
        while ticks < 3000 && (ticks < 2 || b.unit(UnitId(0)).unwrap().state != MoveState::Idle) {
            b.step();
            ticks += 1;
        }
        assert!(ticks < 3000, "{what}: the squad arrives");
        let searches = b.route_searches(Side::Blue);
        assert!(
            searches <= 1 + soldiers,
            "{what}: {searches} searches for one order of {soldiers} soldiers"
        );
    }
}

/// No soldier's body ever overlaps another's (Q10), even where a whole
/// squad files through a gap one man wide.
#[test]
fn soldiers_never_overlap_filing_through_a_one_man_gap() {
    let wall = |y: f64| serde_json::json!({ "kind": "wall", "center": [60, y], "yaw": 0, "half_extents": [0.5, 19.625, 1.5] });
    let mut b = one_squad(
        serde_json::json!([wall(19.625), wall(60.375)]),
        [20.0, 40.0],
        [100.0, 40.0],
    );
    let apart = 2.0 * common::physics("soldier_radius_m");
    for _ in 0..1500 {
        b.step();
        let at: Vec<_> = b.unit(UnitId(0)).unwrap().member_positions().collect();
        for (i, p) in at.iter().enumerate() {
            for q in &at[i + 1..] {
                assert!(
                    (p.xy() - q.xy()).length() >= apart - 1e-6,
                    "overlap at tick {}",
                    b.tick()
                );
            }
        }
    }
    assert_eq!(
        b.unit(UnitId(0)).unwrap().state,
        MoveState::Idle,
        "through and settled"
    );
}
