//! The order queue: what a unit does after its current order, and when
//! that order ends. Driven through real scenarios and commands.
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::units::UnitOrder;

use crate::common::{self, Commander};

mod jets;

const BUILDING: u32 = 0;

fn town(units: Value) -> Battle {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, "china-apartment-point-20x20-7f", [400.0, 300.0], 0.0)] })
    .to_string();
    Battle::new(&common::scenario(&map, units, json!([])), 1)
}

fn ticks(seconds: f64) -> u64 {
    (seconds * f64::from(common::tick_hz())).round() as u64
}

fn until(b: &mut Battle, limit: u64, what: &str, mut test: impl FnMut(&Battle) -> bool) {
    for _ in 0..limit {
        if test(b) {
            return;
        }
        b.step();
    }
    panic!("{what} within {limit} ticks");
}

fn alive(b: &Battle, id: u32) -> bool {
    b.unit(UnitId(id)).is_some_and(|u| u.alive())
}

/// A garrisoned squad's attack ends once the enemy is dead: the attack
/// ends from inside the building as it does anywhere else.
#[test]
fn a_garrisoned_attack_ends_when_its_target_dies() {
    let mut b = town(json!([
        { "side": "blue", "kind": "test_rifle", "position": [350.0, 300.0], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_rifle", "position": [470.0, 300.0], "engagement": "return_fire_only" },
    ]));
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::OccupyBuilding {
            units: vec![UnitId(0)],
            building: BUILDING,
            gesture: 0,
        },
    );
    until(&mut b, ticks(90.0), "the squad inside", |b| {
        b.unit(UnitId(0)).unwrap().garrisoned()
    });
    until(&mut b, ticks(30.0), "the enemy seen", |b| {
        !b.observe(Side::Blue).identified.is_empty()
    });
    let enemy = b.observe(Side::Blue).identified[0].id;
    c.ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: enemy },
        },
    );
    until(&mut b, ticks(240.0), "the enemy dead", |b| !alive(b, 1));
    until(&mut b, ticks(5.0), "the attack over", |b| {
        b.unit(UnitId(0)).unwrap().orders.is_empty()
    });
}

/// A unit driving home keeps going home when an order is queued behind
/// it: a queued order waits its turn and never cancels what the unit is
/// doing now, and nothing follows a unit that leaves.
#[test]
fn a_queued_order_does_not_cancel_a_drive_home() {
    use crate::transport::{side_command, transport_skirmish_setup};
    use contract::command::CommandEnvelope;
    let mut b = Battle::new(&transport_skirmish_setup(), 41);
    let buy = side_command(Side::Blue, 1, Order::purchase("test_apc", [300.0, 100.0]));
    assert_eq!(b.accept(buy).error, None);
    b.accept(side_command(Side::Blue, 2, Order::Ready));
    b.accept(side_command(Side::Red, 1, Order::Ready));
    until(&mut b, ticks(30.0), "the carrier arrived", |b| {
        b.unit(UnitId(0)).is_some()
    });
    let out = [300.0, 300.0];
    assert_eq!(
        b.accept(side_command(Side::Blue, 3, drive(out))).error,
        None
    );
    until(&mut b, ticks(60.0), "the carrier out on the field", |b| {
        let at = b.unit(UnitId(0)).unwrap().position.xy();
        (at - sim::math::v2(out[0], out[1])).length() < 10.0
    });
    let home = Order::Refund {
        units: vec![UnitId(0)],
    };
    assert_eq!(b.accept(side_command(Side::Blue, 4, home)).error, None);
    b.step();
    let onward = CommandEnvelope {
        queued: true,
        ..side_command(Side::Blue, 5, drive(out))
    };
    assert_eq!(b.accept(onward).error, None);
    until(&mut b, ticks(120.0), "the carrier gone home", |b| {
        b.unit(UnitId(0)).unwrap().retired
    });
}

fn drive(goal: [f64; 2]) -> Order {
    Order::Move {
        units: vec![UnitId(0)],
        gesture: 1,
        goal,
        route: contract::command::RoutePolicy::Shortest,
        direction: contract::command::MoveDirection::Forward,
        facing: None,
    }
}

fn field(units: Value) -> Battle {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [] })
    .to_string();
    Battle::new(&common::scenario(&map, units, json!([])), 1)
}

fn seen(b: &mut Battle) -> contract::observation::ObservedTargetId {
    until(b, ticks(30.0), "the enemy seen", |b| {
        !b.observe(Side::Blue).identified.is_empty()
    });
    b.observe(Side::Blue).identified[0].id
}

fn attack(unit: u32, id: contract::observation::ObservedTargetId) -> Order {
    Order::Attack {
        units: vec![UnitId(unit)],
        target: TargetRef::Identified { id },
    }
}

fn near(b: &Battle, unit: u32, goal: [f64; 2], within: f64) -> bool {
    let at = b.unit(UnitId(unit)).unwrap().position.xy();
    (at - sim::math::v2(goal[0], goal[1])).length() < within
}

/// A move shift-queued behind an attack is placed, and the unit drives
/// there once the attack is over.
#[test]
fn a_move_queued_behind_an_attack_runs() {
    let mut b = field(json!([
        { "side": "blue", "kind": "test_tank", "position": [200.0, 300.0], "yaw": 0.0 },
        { "side": "red", "kind": "test_jeep", "position": [420.0, 300.0], "engagement": "return_fire_only" },
    ]));
    let mut c = Commander::new();
    let enemy = seen(&mut b);
    c.ok(&mut b, Side::Blue, attack(0, enemy));
    // The attack is in the unit's queue before the move is placed behind it.
    b.step();
    let goal = [200.0, 480.0];
    let queued = contract::command::CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 2,
        order: drive(goal),
        queued: true,
    };
    let ack = b.accept(queued);
    assert_eq!(ack.error, None);
    assert!(
        ack.placement.unwrap().destinations[0].placed,
        "the queued move is placed"
    );
    until(&mut b, ticks(240.0), "the tank at the queued goal", |b| {
        near(b, 0, goal, 10.0)
    });
}

/// An attack queued behind a move leaves the unit's fire policy alone until
/// the move is done: a queued order never acts before its turn.
#[test]
fn a_queued_attack_waits_its_turn_to_choose_fire_at_will() {
    use contract::command::Engagement;
    let mut b = field(json!([
        { "side": "blue", "kind": "test_tank", "position": [200.0, 300.0], "yaw": 0.0, "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_jeep", "position": [420.0, 300.0], "engagement": "return_fire_only" },
    ]));
    let mut c = Commander::new();
    let enemy = seen(&mut b);
    let goal = [200.0, 420.0];
    c.ok(&mut b, Side::Blue, drive(goal));
    assert_eq!(c.send(&mut b, Side::Blue, attack(0, enemy), true), None);
    b.step();
    let engagement = |b: &Battle| b.unit(UnitId(0)).unwrap().engagement;
    until(&mut b, ticks(120.0), "the move done", |b| {
        assert_ne!(
            (engagement(b), near(b, 0, goal, 3.0)),
            (Engagement::FireAtWill, false),
            "fire at will before the attack's turn"
        );
        !matches!(
            b.unit(UnitId(0)).unwrap().orders.front(),
            Some(UnitOrder::Move(_))
        )
    });
    until(
        &mut b,
        ticks(2.0),
        "fire at will on the attack's turn",
        |b| engagement(b) == Engagement::FireAtWill,
    );
}

/// A skirmish where blue buys one `blue` (`unit`) and a rifle squad that
/// spots for it (and holds the field for blue while a jet flies), and red
/// buys two rifle squads that can't hurt it; orders go through each side's
/// player.
struct Skirmish {
    b: Battle,
    seq: [u64; 2],
    unit: UnitId,
}

impl Skirmish {
    /// Positions are fractions of the map, whose size is `size`: a jet needs
    /// room to fly its runs.
    fn new(blue: &str, category: &str, size: [f64; 2]) -> Self {
        let mut rules = sim::fixtures::test_rules();
        let [w, h] = size;
        let at = |fx: f64, fy: f64| [w * fx, h * fy];
        let roster = [
            (blue, json!(["us"]), category),
            ("test_rifle", json!(["us", "eastern"]), "inf"),
        ];
        for (kind, factions, category) in roster {
            sim::fixtures::patch_catalog(
                &mut rules,
                "units",
                kind,
                json!({"roster":{"factions":factions,"category":category,"family_name":"Test","variant":kind}}),
            );
        }
        let site = |id: &str, p: [f64; 2], kind: &str, counterpart: Value| json!({"id":id,"center":p,"radius_m":40,"kind":kind,"counterpart":counterpart});
        let setup = serde_json::from_value(json!({
            "map":{"size":size,"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]},
            "rules":rules,"units":[],"events":[],"scripts":[],
            "players": common::human_duel(),
            "skirmish":{"rules":{"credits_per_minute":200,"starting_minutes":20,"preparation_s":0,"max_units":30,"max_players_per_side":10,"dispatch_interval_s":1,"capture_s":20,"majority_s":1800,"victory_score":1000},
                "sites":{"entries":[{"side":"blue","center":[w / 2.0, 10.0],"yaw":0.0},{"side":"red","center":[w / 2.0, h - 10.0],"yaw":0.0}],
                "objectives":[site("center", at(0.5, 0.5), "junction", Value::Null),
                    site("south", at(0.2, 0.75), "field", json!("north")),
                    site("north", at(0.8, 0.75), "field", json!("south"))]}}
        }))
        .unwrap();
        let mut s = Skirmish {
            b: Battle::new(&setup, 3),
            seq: [0, 0],
            unit: UnitId(0),
        };
        s.ok(
            Side::Blue,
            Order::purchase("test_rifle", at(0.375, 0.07)),
            false,
        );
        s.ok(
            Side::Red,
            Order::purchase("test_rifle", at(0.375, 0.9)),
            false,
        );
        s.ok(
            Side::Red,
            Order::purchase("test_rifle", at(0.625, 0.9)),
            false,
        );
        s.ok(Side::Blue, Order::Ready, false);
        s.ok(Side::Red, Order::Ready, false);
        until(&mut s.b, ticks(60.0), "every purchase on the field", |b| {
            b.observe(Side::Blue).own.len() == 1 && b.observe(Side::Red).own.len() == 2
        });
        // Nobody fires until blue's attack order chooses fire at will.
        for side in [Side::Blue, Side::Red] {
            let hold = Order::SetEngagement {
                units: s.own(side),
                policy: contract::command::Engagement::ReturnFireOnly,
            };
            s.ok(side, hold, false);
        }
        let squads = s.own(Side::Red);
        for (&q, p) in squads.iter().zip([at(0.4125, 0.7), at(0.5875, 0.7)]) {
            s.ok(Side::Red, move_to(vec![q], p), false);
        }
        // Blue's squad walks up to spot them: a side sees what any of its
        // units see, and a jet overhead can't pick out infantry lying still.
        let spotter = s.own(Side::Blue);
        s.ok(Side::Blue, move_to(spotter.clone(), at(0.5, 0.55)), false);
        s.b.step();
        until(&mut s.b, ticks(600.0), "everyone in place", |b| {
            squads
                .iter()
                .chain(&spotter)
                .all(|&q| b.unit(q).unwrap().orders.is_empty())
        });
        // The unit under test comes on last, fresh (a jet's fuel runs from
        // when it arrives).
        s.ok(Side::Blue, Order::purchase(blue, at(0.5, 0.1)), false);
        until(&mut s.b, ticks(60.0), "the unit on the field", |b| {
            b.observe(Side::Blue).own.len() == 2
        });
        let rifle = common::unit_kind("test_rifle");
        s.unit = (s.b.observe(Side::Blue).own.iter())
            .find(|u| u.kind != rifle)
            .unwrap()
            .id;
        let hold = Order::SetEngagement {
            units: vec![s.unit],
            policy: contract::command::Engagement::ReturnFireOnly,
        };
        s.ok(Side::Blue, hold, false);
        s
    }

    fn ok(&mut self, side: Side, order: Order, queued: bool) {
        self.seq[side.index()] += 1;
        let command = contract::command::CommandEnvelope {
            queued,
            ..crate::transport::side_command(side, self.seq[side.index()], order)
        };
        assert_eq!(self.b.accept(command).error, None);
    }

    fn own(&self, side: Side) -> Vec<UnitId> {
        self.b.observe(side).own.iter().map(|u| u.id).collect()
    }

    /// Blue's view of the red squad standing nearest `p`.
    fn seen_near(&mut self, p: [f64; 2]) -> contract::observation::ObservedTargetId {
        until(&mut self.b, ticks(30.0), "both squads seen", |b| {
            b.observe(Side::Blue).identified.len() == 2
        });
        let d = |t: &contract::observation::IdentifiedUnit| {
            (t.position[0] - p[0]).hypot(t.position[1] - p[1])
        };
        let seen = &self.b.observe(Side::Blue).identified;
        seen.iter().min_by(|a, c| d(a).total_cmp(&d(c))).unwrap().id
    }
}

fn move_to(units: Vec<UnitId>, goal: [f64; 2]) -> Order {
    Order::Move {
        units,
        gesture: 1,
        goal,
        route: contract::command::RoutePolicy::Shortest,
        direction: contract::command::MoveDirection::Forward,
        facing: None,
    }
}

/// One stretch of a unit doing one kind of order: the rounds it had fired
/// as it began, and for an attack whether its target was alive as it began
/// and as it handed off.
#[derive(Debug)]
struct Stretch {
    kind: &'static str,
    shots: u32,
    alive_at_start: bool,
    alive_at_end: bool,
}

/// What a unit was doing, tick by tick, as stretches of one kind of order.
fn follow(b: &mut Battle, unit: UnitId, limit: u64) -> Vec<Stretch> {
    let mut seen: Vec<Stretch> = Vec::new();
    for _ in 0..limit {
        let Some(u) = b.unit(unit).filter(|u| !u.retired) else {
            return seen;
        };
        let shots: u32 = u.mounts.iter().map(|m| m.shots).sum();
        let (kind, alive) = match u.orders.front() {
            Some(UnitOrder::Attack { target, .. }) => (
                "attack",
                match target {
                    sim::weapons::Target::Unit(id) => b.unit(*id).is_some_and(|t| t.alive()),
                    _ => true,
                },
            ),
            Some(UnitOrder::Withdraw) => ("home", true),
            Some(UnitOrder::Move(_)) if u.withdrawing => ("home", true),
            Some(UnitOrder::Move(_)) => ("move", true),
            Some(_) => ("other", true),
            None => ("idle", true),
        };
        match seen.last_mut() {
            Some(last) if last.kind == kind => last.alive_at_end = alive,
            _ => seen.push(Stretch {
                kind,
                shots,
                alive_at_start: alive,
                alive_at_end: alive,
            }),
        }
        b.step();
    }
    panic!("still on the field after {limit} ticks: {seen:?}");
}

/// The acceptance queue: attack → move → attack the same enemy → move →
/// attack another → return to base, all shift-queued. Each attack fires a
/// volley and hands off; every move is driven; the unit goes home.
#[test]
fn a_ground_unit_works_through_a_queue_of_attacks_moves_and_home() {
    let mut s = Skirmish::new("test_tank", "veh", [800.0, 600.0]);
    let tank = s.unit;
    s.ok(Side::Blue, move_to(vec![tank], [400.0, 280.0]), false);
    s.b.step();
    until(&mut s.b, ticks(120.0), "the tank in place", |b| {
        b.unit(tank).unwrap().orders.is_empty()
    });
    let a = s.seen_near([330.0, 420.0]);
    let other = s.seen_near([470.0, 420.0]);
    let attack = |id| Order::Attack {
        units: vec![tank],
        target: TargetRef::Identified { id },
    };
    s.ok(Side::Blue, attack(a), false);
    s.ok(Side::Blue, move_to(vec![tank], [350.0, 300.0]), true);
    s.ok(Side::Blue, attack(a), true);
    s.ok(Side::Blue, move_to(vec![tank], [450.0, 300.0]), true);
    s.ok(Side::Blue, attack(other), true);
    s.ok(Side::Blue, Order::Refund { units: vec![tank] }, true);
    s.b.step();
    let seen = follow(&mut s.b, tank, ticks(300.0));
    let kinds: Vec<_> = seen.iter().map(|s| s.kind).collect();
    assert_eq!(
        kinds,
        ["attack", "move", "attack", "move", "attack", "home"],
        "{seen:?}"
    );
    // The first attack fired a volley and handed off: the squad it hit
    // was still alive for the second attack on it.
    assert!(
        seen[0].alive_at_end,
        "the first attack fought on until its target died: {seen:?}"
    );
    // An attack on a living target fires before it hands off; one whose
    // target died meanwhile ends at once.
    for w in (seen.windows(2)).filter(|w| w[0].kind == "attack" && w[0].alive_at_start) {
        assert!(
            w[1].shots > w[0].shots,
            "an attack handed off without firing: {seen:?}"
        );
    }
}

fn ground_at(unit: u32, p: [f64; 2]) -> Order {
    Order::Attack {
        units: vec![UnitId(unit)],
        target: TargetRef::Ground {
            point: [p[0], p[1], 0.0],
        },
    }
}

/// The rounds the launcher's own mount has fired.
fn salvo(b: &Battle) -> u32 {
    b.unit(UnitId(0)).unwrap().mounts[0].shots
}

fn attacking(b: &Battle, unit: u32) -> bool {
    matches!(
        b.unit(UnitId(unit)).unwrap().orders.front(),
        Some(UnitOrder::Attack { .. })
    )
}

/// The test tank with its main gun reshaped into a salvo launcher: HE in a
/// magazine of `rounds` with no burst and a long reload. (Its HMG stays.)
fn launcher(rounds: u32) -> Battle {
    let mut rules = sim::fixtures::test_rules();
    rules["weapons"]["tank_he"]["magazine"] = json!({ "rounds": rounds, "shot_interval_s": 0.3 });
    rules["weapons"]["tank_he"]["reload_s"] = json!(60.0);
    rules["weapons"]["tank_he"]["ammo"] = json!(100);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({ "mounts": [{ "id": "cannon", "name": "Launcher", "weapons": ["tank_he"],
            "turret": true, "pivot_m": [0, 0, 1.45], "muzzle_m": [5.9, 0, 0.55] }] }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [{ "side": "blue", "kind": "test_tank", "position": [200.0, 300.0], "yaw": 0.0 }],
        "events": [], "scripts": []
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

/// A launcher whose magazine has no burst fires the whole magazine as its
/// volley, then drives the move queued behind it.
#[test]
fn a_salvo_launcher_fires_its_whole_magazine_as_one_volley() {
    let mut b = launcher(4);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, ground_at(0, [400.0, 300.0]));
    assert_eq!(
        c.send(&mut b, Side::Blue, drive([200.0, 450.0]), true),
        None
    );
    b.step();
    until(&mut b, ticks(60.0), "the attack handed off", |b| {
        !attacking(b, 0)
    });
    assert_eq!(salvo(&b), 4);
}

/// Queued mid-salvo, the launcher finishes the magazine it is firing: the
/// cycle under way is the volley.
#[test]
fn queued_mid_salvo_a_launcher_fires_what_is_left() {
    let mut b = launcher(4);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, ground_at(0, [400.0, 300.0]));
    until(&mut b, ticks(60.0), "the salvo begun", |b| salvo(b) == 1);
    assert_eq!(
        c.send(&mut b, Side::Blue, drive([200.0, 450.0]), true),
        None
    );
    b.step();
    until(&mut b, ticks(60.0), "the attack handed off", |b| {
        !attacking(b, 0)
    });
    assert_eq!(salvo(&b), 4);
}

/// The last attack-ground in a queue keeps firing salvo after salvo.
#[test]
fn the_last_attack_on_the_ground_keeps_firing() {
    let mut b = launcher(2);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, ground_at(0, [400.0, 300.0]));
    until(&mut b, ticks(240.0), "three salvos", |b| salvo(b) >= 6);
    assert!(attacking(&b, 0));
}

/// Plain riflemen told to attack a tank, with a move queued, hand off at once:
/// nothing they carry can hurt it.
#[test]
fn an_attack_nothing_can_hurt_hands_off_at_once() {
    let mut b = field(json!([
        { "side": "blue", "kind": "test_smoke_squad", "position": [300.0, 300.0], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_tank", "position": [420.0, 300.0], "engagement": "return_fire_only" },
    ]));
    let mut c = Commander::new();
    let enemy = seen(&mut b);
    c.ok(&mut b, Side::Blue, attack(0, enemy));
    assert_eq!(
        c.send(&mut b, Side::Blue, drive([300.0, 450.0]), true),
        None
    );
    b.step();
    until(&mut b, 3, "the attack handed off", |b| !attacking(b, 0));
}

/// An attack on ground out of every gun's reach closes on its aim and fires
/// its volley before an order queued behind it takes over; as the last
/// order, it closes and keeps firing.
#[test]
fn an_attack_on_ground_out_of_reach_closes_and_fires_before_handing_off() {
    let map = json!({ "size": [3000, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [] })
    .to_string();
    let units =
        json!([{ "side": "blue", "kind": "test_tank", "position": [200.0, 300.0], "yaw": 0.0 }]);
    let rules = common::rules();
    let tank = rules.catalog.index("test_tank").unwrap();
    let (_, reach) = sim::strike::reach_m(&rules, &sim::weapons::Arsenal::new(&rules), tank)
        .expect("the tank strikes an area");
    let far = [200.0 + reach + 300.0, 300.0];
    let mut last = Battle::new(&common::scenario(&map, units.clone(), json!([])), 1);
    Commander::new().ok(&mut last, Side::Blue, ground_at(0, far));
    // Its rounds from any gun: the first to come in reach fires first.
    let fired = |b: &Battle| {
        b.unit(UnitId(0))
            .unwrap()
            .mounts
            .iter()
            .map(|m| m.shots)
            .sum::<u32>()
    };
    until(
        &mut last,
        ticks(90.0),
        "the last attack closed and fired",
        |b| fired(b) > 0,
    );
    for _ in 0..ticks(10.0) {
        last.step();
    }
    assert!(attacking(&last, 0), "the last attack fights on");
    let mut b = Battle::new(&common::scenario(&map, units, json!([])), 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, ground_at(0, far));
    assert_eq!(
        c.send(&mut b, Side::Blue, drive([200.0, 450.0]), true),
        None
    );
    b.step();
    until(&mut b, ticks(90.0), "the attack handed off", |b| {
        !attacking(b, 0)
    });
    assert!(fired(&b) > 0, "it handed off without firing");
}

/// A garrison fires from its building or not at all: told to strike ground
/// past every gun's reach with an order queued behind, it has nothing to do
/// there and hands off at once, where a unit in the open closes first.
#[test]
fn a_garrison_hands_off_a_ground_aim_out_of_reach() {
    let mut b = town(json!([{ "side": "blue", "kind": "test_rifle", "position": [350.0, 300.0] }]));
    let mut c = Commander::new();
    let inside = |b: &Battle| b.unit(UnitId(0)).unwrap().garrisoned();
    let enter = Order::OccupyBuilding {
        units: vec![UnitId(0)],
        building: BUILDING,
        gesture: 0,
    };
    c.ok(&mut b, Side::Blue, enter);
    until(&mut b, ticks(90.0), "the squad inside", inside);
    // The map's far corner: beyond a rifle's reach from the building.
    c.ok(&mut b, Side::Blue, ground_at(0, [790.0, 590.0]));
    let leave = Order::ExitBuilding {
        units: vec![UnitId(0)],
    };
    assert_eq!(c.send(&mut b, Side::Blue, leave, true), None);
    until(&mut b, ticks(20.0), "out of the building", |b| !inside(b));
}

/// An anti-tank team told to attack a vehicle, with a move queued, stays
/// with its guided missile until it lands before moving on: a launcher that
/// walked off would drop it.
#[test]
fn a_guided_missile_is_flown_home_before_the_attack_hands_off() {
    let mut b = field(json!([
        { "side": "blue", "kind": "test_at", "position": [200.0, 300.0], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_jeep", "position": [600.0, 300.0], "engagement": "return_fire_only" },
    ]));
    let mut c = Commander::new();
    let enemy = seen(&mut b);
    c.ok(&mut b, Side::Blue, attack(0, enemy));
    assert_eq!(
        c.send(&mut b, Side::Blue, drive([200.0, 450.0]), true),
        None
    );
    b.step();
    let guiding = |b: &Battle| {
        b.unit(UnitId(0))
            .unwrap()
            .mounts
            .iter()
            .any(|m| m.support.is_some())
    };
    until(&mut b, ticks(60.0), "a missile in flight", guiding);
    until(&mut b, ticks(60.0), "the attack handed off", |b| {
        assert!(
            attacking(b, 0) || !guiding(b),
            "handed off with a missile in flight"
        );
        !attacking(b, 0)
    });
}

/// A launcher that must stand still to fire, sent to a firing spot with an
/// attack on the ground and a move queued behind, stops there and fires its
/// salvo before it moves on: setting up after the drive doesn't count as
/// being unable to fire.
#[test]
fn a_launcher_queued_through_an_attack_on_the_ground_fires_on_the_way() {
    let mut rules = sim::fixtures::test_rules();
    rules["weapons"]["tank_he"]["magazine"] = json!({ "rounds": 4, "shot_interval_s": 0.3 });
    rules["weapons"]["tank_he"]["reload_s"] = json!(60.0);
    rules["weapons"]["tank_he"]["ammo"] = json!(100);
    rules["weapons"]["tank_he"]["stationary"] = json!(true);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({ "mounts": [{ "id": "cannon", "name": "Launcher", "weapons": ["tank_he"],
            "turret": true, "pivot_m": [0, 0, 1.45], "muzzle_m": [5.9, 0, 0.55] }] }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [{ "side": "blue", "kind": "test_tank", "position": [150.0, 300.0], "yaw": 0.0 }],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, drive([250.0, 300.0]));
    assert_eq!(
        c.send(&mut b, Side::Blue, ground_at(0, [450.0, 300.0]), true),
        None
    );
    assert_eq!(
        c.send(&mut b, Side::Blue, drive([250.0, 450.0]), true),
        None
    );
    b.step();
    until(&mut b, ticks(60.0), "the attack's turn", |b| {
        attacking(b, 0)
    });
    until(&mut b, ticks(60.0), "the attack handed off", |b| {
        !attacking(b, 0)
    });
    assert!(salvo(&b) > 0, "it moved on without firing");
}
