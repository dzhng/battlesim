//! Order markers and the Space overlay (D2, D2+, Q9, Q31): the
//! publication carries each own soldier's resolved spot and cover and each
//! unit's final facing, a right-drag's facing is kept at the end of the
//! move, and nothing of the enemy's plan reaches the other side.
use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{v2, wrap_angle};
use sim::publication::Publisher;

fn rules() -> Value {
    sim::fixtures::test_rules()
}

fn setup(size: [f64; 2], props: Value, units: Value, scripts: Value) -> ScenarioDefinition {
    serde_json::from_value(json!({
        "map": { "size": size, "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props },
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
fn an_oblique_squad_preview_keeps_the_committed_approach_heading() {
    use contract::command::{CommandEnvelope, MovePreviewRequest, Order};
    let mut b = Battle::new(
        &setup(
            [160.0, 120.0],
            json!([]),
            json!([{ "side": "blue", "kind": "test_rifle", "position": [30, 60], "engagement": "return_fire_only" }]),
            json!([]),
        ),
        1,
    );
    let request = MovePreviewRequest {
        units: vec![UnitId(0)],
        goal: [100.0, 67.0],
        ..Default::default()
    };
    let preview = b.preview_move(Side::Blue, &request).unwrap();
    assert!(preview[0].placed);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: request.units,
            goal: request.goal,
            gesture: 1,
            route: request.route,
            direction: request.direction,
            facing: request.facing,
        },
    });
    assert_eq!(ack.error, None);
    for _ in 0..30 {
        b.step();
        if !own(&b, 0).route.is_empty() {
            break;
        }
    }
    let committed = own(&b, 0);
    assert!(committed.goal.is_some());
    assert!(!committed.route.is_empty());
    assert!(
        wrap_angle(preview[0].facing - committed.final_facing).abs() < 1e-6,
        "preview {} differs from committed {}",
        preview[0].facing,
        committed.final_facing
    );
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
            { "side": "blue", "kind": "test_rifle", "position": [40, 60], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [200, 60], "engagement": "return_fire_only" },
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
fn a_right_drag_facing_is_published_and_kept_by_a_squad_a_tank_and_wheels_driving_in_along_it() {
    let north = std::f64::consts::FRAC_PI_2;
    let s = setup(
        [200.0, 200.0],
        json!([]),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [40, 40], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_tank", "position": [40, 100], "yaw": 0, "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_jeep", "position": [40, 160], "yaw": 0, "engagement": "return_fire_only" },
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
    assert!((own(&b, 1).final_facing - north).abs() < 1e-9, "test_tank");
    assert!(
        wrap_angle(own(&b, 2).final_facing - north).abs() < 1e-6,
        "the jeep's marker shows the heading it lines up on"
    );
    let mut reversed = false;
    let mut arrived = None;
    for _ in 0..40 * 30 {
        b.step();
        let jeep = b.unit(UnitId(2)).unwrap();
        reversed |= jeep.reversing;
        if arrived.is_none() && jeep.orders.is_empty() {
            arrived = Some(jeep.yaw);
        }
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
    let jeep = b.unit(UnitId(2)).unwrap();
    assert!(
        (jeep.position.xy() - v2(100.0, 160.0)).length() < 3.0,
        "the jeep reached its spot: {:?} yaw {} state {:?} route {:?}",
        jeep.position,
        jeep.yaw,
        jeep.state,
        jeep.route
    );
    let arrived = arrived.expect("the jeep finished its move");
    assert!(
        wrap_angle(arrived - north).abs() < 0.35,
        "the jeep drove in facing the drag: {arrived}"
    );
    assert!(
        wrap_angle(yaw(2) - north).abs() < 1e-6,
        "and turned the rest of the way on the spot: {}",
        yaw(2)
    );
    assert!(!reversed, "lining up never backs up");
}

/// A wheeled hull lines up as near the drag as a forward turn allows, then
/// turns the rest of the way on the spot: a facing straight back the way it
/// came arrives square to its approach, never backing and filling; with no
/// room for the lead-in it arrives the way it came.
#[test]
fn wheels_line_up_as_far_as_driving_forward_allows_then_turn_on_the_spot() {
    let north = std::f64::consts::FRAC_PI_2;
    let west = std::f64::consts::PI;
    let s = setup(
        [200.0, 200.0],
        json!([]),
        json!([
            { "side": "blue", "kind": "test_jeep", "position": [40, 100], "yaw": 0, "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_jeep", "position": [40, 30], "yaw": 0, "engagement": "return_fire_only" },
        ]),
        json!([
            mv(1, "blue", 0, [120.0, 100.0], json!({ "facing": west })),
            // The lead-in would start off the map's southern edge.
            mv(1, "blue", 1, [120.0, 6.0], json!({ "facing": north })),
        ]),
    );
    let mut b = Battle::new(&s, 2);
    let mut reversed = [false; 2];
    let mut arrived = [None; 2];
    for _ in 0..40 * 30 {
        b.step();
        for id in 0..2 {
            let jeep = b.unit(UnitId(id as u32)).unwrap();
            reversed[id] |= jeep.reversing;
            if arrived[id].is_none() && jeep.orders.is_empty() {
                arrived[id] = Some(jeep.yaw);
            }
        }
    }
    let unit = |id: u32| b.unit(UnitId(id)).unwrap();
    let [Some(square), Some(came)] = arrived else {
        panic!("both jeeps finish their moves: {arrived:?}");
    };
    assert!((unit(0).position.xy() - v2(120.0, 100.0)).length() < 3.0);
    assert!(
        (wrap_angle(square).abs() - std::f64::consts::FRAC_PI_2).abs() < 0.35,
        "arrives square to its approach, not turned round: {square}"
    );
    assert!((unit(1).position.xy() - v2(120.0, 6.0)).length() < 3.0);
    assert!(
        wrap_angle(came - north).abs() > 0.5,
        "no room to line up: it arrives the way it came, {came}"
    );
    assert!(
        wrap_angle(unit(0).yaw - west).abs() < 1e-6,
        "then turns on the spot to the drag: {}",
        unit(0).yaw
    );
    assert!(
        wrap_angle(unit(1).yaw - north).abs() < 1e-6,
        "then turns on the spot to the drag: {}",
        unit(1).yaw
    );
    assert_eq!(reversed, [false; 2], "lining up never backs up");
}

/// A short right-drag a wheeled hull cannot line up on, with no room to
/// drive out and back in: a spot inside its own turning circle, and one just
/// ahead whose straight lead-in would start behind it. Each drives there as
/// best it can, then turns on the spot to the drag, and stays put.
#[test]
fn wheels_finish_a_short_right_drag_by_turning_on_the_spot() {
    let east = 0.0;
    let south_west = -3.0 * std::f64::consts::FRAC_PI_4;
    let s = setup(
        [300.0, 300.0],
        json!([]),
        json!([
            { "side": "blue", "kind": "test_jeep", "position": [100, 100], "yaw": 0, "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_jeep", "position": [100, 200], "yaw": 0, "engagement": "return_fire_only" },
        ]),
        json!([
            // Behind and to the right, inside the circle a right turn sweeps.
            mv(1, "blue", 0, [96.0, 95.0], json!({ "facing": south_west })),
            // Ten metres ahead, facing on: the lead-in would start behind it.
            mv(1, "blue", 1, [110.0, 200.0], json!({ "facing": east })),
        ]),
    );
    let mut b = Battle::new(&s, 3);
    let goals = [(v2(96.0, 95.0), south_west), (v2(110.0, 200.0), east)];
    let done = |b: &Battle, id: u32| {
        let u = b.unit(UnitId(id)).unwrap();
        u.orders.is_empty() && u.route.is_none() && u.turn_to.is_none()
    };
    for _ in 0..30 * 30 {
        b.step();
        if b.tick() > 3 && done(&b, 0) && done(&b, 1) {
            break;
        }
    }
    let rest: Vec<_> = (0..2)
        .map(|id| b.unit(UnitId(id)).unwrap().position)
        .collect();
    for _ in 0..5 * 30 {
        b.step();
    }
    for (id, (goal, facing)) in goals.into_iter().enumerate() {
        let u = b.unit(UnitId(id as u32)).unwrap();
        assert!(
            done(&b, id as u32),
            "jeep {id} still working its order: at {:?} yaw {} state {:?} route {:?}",
            u.position,
            u.yaw,
            u.state,
            u.route
        );
        assert!(
            (u.position.xy() - goal).length() < 3.0,
            "jeep {id} reached its spot: {:?}",
            u.position
        );
        assert!(
            wrap_angle(u.yaw - facing).abs() < 0.02,
            "jeep {id} turned on the spot to the drag: {}",
            u.yaw
        );
        assert!(
            (u.position - rest[id]).xy().length() < 1e-9,
            "jeep {id} stays put once done"
        );
    }
}

#[test]
fn a_reversing_units_final_marker_shows_its_held_facing() {
    let s = setup(
        [200.0, 120.0],
        json!([]),
        json!([{ "side": "blue", "kind": "test_tank", "position": [120, 60], "yaw": 0,
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
                .unwrap()
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
        { "side": "blue", "kind": "test_rifle", "position": [40, 60], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_tank", "position": [1150, 60], "engagement": "return_fire_only" },
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

/// A U.S. against Eastern skirmish on a map with a building (prop 0, turned
/// off the world's axes): blue
/// buys a `blue` and red a tank, and both drive in, holding their fire,
/// and stand. A second blue player, who buys nothing, gives each side two
/// roads in, blue's one by each side edge (`WEST`, `EAST`). Its commands go
/// out in each side's sequence, each admitted.
const WEST: [f64; 2] = [10.0, 150.0];
const EAST: [f64; 2] = [790.0, 150.0];

struct Skirmish {
    b: Battle,
    seq: [u64; 2],
}

impl Skirmish {
    fn new(blue: &str) -> Self {
        use contract::command::Order;
        let mut rules = rules();
        for (kind, faction, category) in [(blue, "us", "inf"), ("test_tank", "eastern", "veh")] {
            sim::fixtures::patch_catalog(
                &mut rules,
                "units",
                kind,
                json!({ "cost": 100,
                "roster": { "factions": [faction], "category": category, "family_name": "Test", "variant": "Test" } }),
            );
        }
        let mut s = crate::skirmish::match_on(rules);
        let house =
            crate::common::building(0, "china-apartment-point-20x20-7f", [500.0, 150.0], 0.5);
        let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35, "props": [], "buildings": [house] });
        s.map = crate::common::scenario_with(&map.to_string(), json!([]), json!([]), json!([])).map;
        s.players.push(
            serde_json::from_value(json!({ "id": "blue-2", "side": "blue",
                "faction": "us", "controller": "human", "human_capable": true }))
            .unwrap(),
        );
        let roads = json!([
            { "side": "blue", "center": WEST, "yaw": 0.0 },
            { "side": "blue", "center": EAST, "yaw": std::f64::consts::PI },
            { "side": "red", "center": [400, 590], "yaw": -std::f64::consts::FRAC_PI_2 },
            { "side": "red", "center": [600, 590], "yaw": -std::f64::consts::FRAC_PI_2 },
        ]);
        s.skirmish.as_mut().unwrap().sites.entries = serde_json::from_value(roads).unwrap();
        let mut m = Skirmish {
            b: Battle::new(&s, 1),
            seq: [0; 2],
        };
        m.order(Side::Blue, Order::purchase(blue, [300.0, 100.0]), false);
        m.order(
            Side::Red,
            Order::purchase("test_tank", [330.0, 180.0]),
            false,
        );
        m.order(Side::Blue, Order::Ready, false);
        let ready = m.b.accept(contract::command::CommandEnvelope {
            side: Side::Blue,
            player: Some("blue-2".into()),
            seq: 1,
            order: Order::Ready,
            queued: false,
        });
        assert_eq!(ready.error, None, "{ready:?}");
        m.order(Side::Red, Order::Ready, false);
        let mut holding = [false; 2];
        for _ in 0..30 * 120 {
            m.b.step();
            // Each holds its fire as it arrives: neither starts a fight.
            for side in [Side::Blue, Side::Red] {
                let Some(u) = m.b.observe(side).own.first().map(|u| u.id) else {
                    continue;
                };
                if !std::mem::replace(&mut holding[side.index()], true) {
                    let policy = contract::command::Engagement::ReturnFireOnly;
                    m.order(
                        side,
                        Order::SetEngagement {
                            units: vec![u],
                            policy,
                        },
                        false,
                    );
                }
            }
            let (blue, red) = (m.b.observe(Side::Blue), m.b.observe(Side::Red));
            let stands = |o: &contract::observation::ObservationFrame| {
                o.own
                    .first()
                    .is_some_and(|u| u.state == MoveState::Idle && u.steps.is_empty())
            };
            if stands(blue) && stands(red) && blue.identified.len() == 1 {
                return m;
            }
        }
        panic!("the two never stood within sight");
    }

    fn order(&mut self, side: Side, order: contract::command::Order, queued: bool) {
        let seq = &mut self.seq[side.index()];
        *seq += 1;
        let player = Some(
            if side == Side::Blue {
                "blue-1"
            } else {
                "red-1"
            }
            .into(),
        );
        let envelope = contract::command::CommandEnvelope {
            side,
            player,
            seq: *seq,
            order,
            queued,
        };
        let ack = self.b.accept(envelope);
        assert_eq!(ack.error, None, "{ack:?}");
    }

    fn own(&self, side: Side) -> &OwnUnit {
        &self.b.observe(side).own[0]
    }

    /// Blue's unit drives through `legs`, then, queued, attacks red's tank.
    fn move_then_attack(&mut self, legs: &[[f64; 2]]) {
        use contract::command::{Order, RoutePolicy, TargetRef};
        let unit = self.own(Side::Blue).id;
        let tank = self.b.observe(Side::Blue).identified[0].id;
        for (k, &goal) in legs.iter().enumerate() {
            let walk = Order::Move {
                units: vec![unit],
                gesture: k as u64 + 1,
                goal,
                route: RoutePolicy::Shortest,
                direction: Default::default(),
                facing: None,
            };
            self.order(Side::Blue, walk, k > 0);
        }
        let target = TargetRef::Identified { id: tank };
        self.order(
            Side::Blue,
            Order::Attack {
                units: vec![unit],
                target,
            },
            true,
        );
    }
}

#[test]
fn every_order_is_published_as_a_step_in_order() {
    use contract::command::{Order, TargetRef};
    use contract::observation::OrderStep;
    let mut m = Skirmish::new("test_rifle");
    let goal = [150.0, 100.0];
    m.move_then_attack(&[goal]);
    let squad = m.own(Side::Blue).id;
    // Behind the attack, a move: it runs once the attack hands off.
    let onward = [200.0, 100.0];
    let drive = Order::Move {
        units: vec![squad],
        gesture: 3,
        goal: onward,
        route: contract::command::RoutePolicy::Shortest,
        direction: contract::command::MoveDirection::Forward,
        facing: None,
    };
    m.order(Side::Blue, drive, true);
    let occupy = Order::OccupyBuilding {
        units: vec![squad],
        building: 0,
        gesture: 2,
    };
    m.order(Side::Blue, occupy, true);
    m.order(Side::Blue, Order::Refund { units: vec![squad] }, true);
    m.b.step();
    let tank = &m.b.observe(Side::Blue).identified[0];
    let [move_step, attack, onward_step, garrison, home] = m.own(Side::Blue).steps[..] else {
        panic!("five steps, got {:?}", m.own(Side::Blue).steps);
    };
    assert_eq!(
        move_step,
        OrderStep::Move {
            point: goal,
            attack_move: false
        }
    );
    assert_eq!(
        attack,
        OrderStep::Attack {
            target: Some(TargetRef::Identified { id: tank.id }),
            point: Some([tank.position[0], tank.position[1]]),
        }
    );
    assert_eq!(
        onward_step,
        OrderStep::Move {
            point: onward,
            attack_move: false
        }
    );
    assert!(
        matches!(garrison, OrderStep::Garrison { building: 0, .. }),
        "{garrison:?}"
    );
    // The way home starts where the garrison ends, by the house: the east
    // road, though the squad stands, and the move before ends, nearer the
    // west one.
    assert_eq!(home, OrderStep::ReturnToBase { point: EAST });
}

#[test]
fn a_queued_attack_step_follows_its_target_holds_through_the_grace_then_names_nothing() {
    use contract::command::{Order, RoutePolicy};
    use contract::observation::OrderStep;
    // An unarmed truck: nothing it does provokes the tank.
    let mut m = Skirmish::new("test_supply");
    // Drives that keep it well away from the tank, then the attack.
    m.move_then_attack(&[[60.0, 100.0], [300.0, 20.0], [60.0, 20.0]]);
    let attack = |m: &Skirmish| {
        let steps = &m.own(Side::Blue).steps;
        assert!(steps.len() > 1, "the attack is still queued");
        match *steps.last().unwrap() {
            OrderStep::Attack { target, point } => (target, point),
            step => panic!("the attack is the last step, got {step:?}"),
        }
    };
    let away = Order::Move {
        units: vec![m.own(Side::Red).id],
        gesture: 9,
        goal: [330.0, 560.0],
        route: RoutePolicy::Shortest,
        direction: Default::default(),
        facing: None,
    };
    m.order(Side::Red, away, false);
    let (mut seen, mut followed, mut held, mut forgotten) = (None, 0, 0, false);
    for _ in 0..30 * 40 {
        m.b.step();
        let (target, point) = attack(&m);
        match m.b.observe(Side::Blue).identified.first() {
            Some(t) => {
                let at = [t.position[0], t.position[1]];
                assert_eq!(point, Some(at), "the point follows the track");
                followed += usize::from(seen.is_some_and(|s| s != at));
                seen = Some(at);
            }
            // Unseen, the side keeps its track through the grace, then
            // forgets the unit: the attack names nothing and knows no spot.
            None if point.is_some() && !forgotten => {
                assert_eq!(point, seen, "the point holds where it was lost");
                held += 1;
            }
            None => {
                assert_eq!(
                    (target, point),
                    (None, None),
                    "once forgotten, nothing comes back"
                );
                forgotten = true;
            }
        }
    }
    assert!(followed > 10, "the target moved while seen");
    assert!(
        held > 0 && forgotten,
        "held through the grace ({held} ticks), then forgotten"
    );
}

#[test]
fn a_garrison_and_an_exit_step_carry_the_box_the_held_building_is_ringed_by() {
    use contract::command::Order;
    use contract::observation::{Footprint, HostRef, OrderStep};
    let mut m = Skirmish::new("test_rifle");
    let squad = m.own(Side::Blue).id;
    let occupy = Order::OccupyBuilding {
        units: vec![squad],
        building: 0,
        gesture: 1,
    };
    m.order(Side::Blue, occupy, false);
    m.b.step();
    let OrderStep::Garrison {
        footprint: Some(queued),
        ..
    } = m.own(Side::Blue).steps[0]
    else {
        panic!(
            "a garrison step with its footprint: {:?}",
            m.own(Side::Blue).steps
        );
    };
    let held = |m: &Skirmish| match m.own(Side::Blue).hold.map(|h| h.host) {
        Some(HostRef::Building { center, half, .. }) => Some(Footprint { center, half }),
        _ => None,
    };
    let mut inside = None;
    for _ in 0..30 * 300 {
        m.b.step();
        inside = held(&m);
        if inside.is_some() && m.own(Side::Blue).steps.is_empty() {
            break;
        }
    }
    // The map's building, standing intact: the box its hold rings.
    assert_eq!(inside, Some(queued), "the step rings what the hold rings");
    m.order(
        Side::Blue,
        Order::ExitBuilding { units: vec![squad] },
        false,
    );
    m.b.step();
    assert_eq!(
        m.own(Side::Blue).steps[0],
        OrderStep::Exit {
            point: queued.center,
            footprint: Some(queued)
        }
    );
}
