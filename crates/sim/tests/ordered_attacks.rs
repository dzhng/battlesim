//! A jet turns to attack only under an attack or attack-move order. Sent
//! somewhere, or left with no order, it keeps circling its point: it never
//! chases an enemy, answers an attacker or starts a run, yet its weapons
//! still fire at what they bear on, under its engagement policy. An explicit
//! attack still runs as ever.
use crate::common::air_field::narrow_field_rules;
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;
use sim::fixtures;

/// A test jet carrying a guided ground missile (it needs no nose-on pass to
/// fire) 400 m from a red tank, firing at will. Nothing in high air sees
/// the ground: with `fire_tick` the
/// tank fires then and the jet's side knows it only as that firing contact;
/// without, a blue recon squad beside the tank spots it for the jet.
fn missile_jet_battle(fire_tick: Option<u64>) -> Battle {
    let mut rules = narrow_field_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        // Mounts merge by id: the missile takes the place of its gun.
        json!({ "mounts": [{
            "id": "gun", "name": "AGM", "weapons": ["air_ground_missile"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }] }),
    );
    let (events, spotter) = if let Some(tick) = fire_tick {
        (json!([{ "tick": tick, "fire": { "unit": 1 } }]), json!([]))
    } else {
        (
            json!([]),
            json!([{ "side": "blue", "kind": "test_recon", "position": [900, 300], "engagement": "return_fire_only" }]),
        )
    };
    let mut setup = json!({
        "map": { "size": [2000, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 400], "yaw": 0.0, "engagement": "fire_at_will" },
            { "side": "red", "kind": "test_tank", "position": [900, 400], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
        ],
        "events": events, "scripts": []
    });
    setup["units"]
        .as_array_mut()
        .unwrap()
        .extend(spotter.as_array().unwrap().iter().cloned());
    Battle::new(&serde_json::from_value(setup).unwrap(), 1)
}

fn order(battle: &mut Battle, order: Order) {
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order,
    });
    assert_eq!(ack.error, None);
}

const TANK: [f64; 2] = [900.0, 400.0];

fn move_to(battle: &mut Battle, goal: [f64; 2]) {
    order(
        battle,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    );
}

fn attack_move_to(battle: &mut Battle, goal: [f64; 2]) {
    order(
        battle,
        Order::AttackMove {
            units: vec![UnitId(0)],
            gesture: 1,
            goal,
        },
    );
}

/// Whether the jet is still on its attack-move, within twice its turn
/// radius of `goal`: it arrived, and the order lives on.
fn hunting_round(battle: &Battle, goal: [f64; 2]) -> bool {
    let jet = battle.unit(UnitId(0)).unwrap();
    let sim::units::Motion::Air(flight) = jet.motion else {
        panic!("test_jet flies");
    };
    let radius = flight.cruise_mps / flight.turn_rad_s;
    let off = libm::hypot(jet.position.x - goal[0], jet.position.y - goal[1]);
    matches!(
        jet.orders.front(),
        Some(sim::units::UnitOrder::AttackMove(_))
    ) && off <= 2.0 * radius
}

/// What a jet did over `ticks`: whether it fired, and whether it began a
/// run or a dogfight (turned to chase).
struct Flown {
    fired: bool,
    chased: bool,
}

fn fly(battle: &mut Battle, ticks: u32) -> Flown {
    let mut flown = Flown {
        fired: false,
        chased: false,
    };
    for _ in 0..ticks {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        let air = jet.air.unwrap();
        flown.fired |= jet.mounts[0].shots > 0;
        flown.chased |= air.pass.is_some() || air.dogfight_target.is_some();
    }
    flown
}

#[test]
fn a_jet_sent_over_a_seen_enemy_circles_and_fires_what_bears_without_a_run() {
    let mut battle = missile_jet_battle(None);
    move_to(&mut battle, TANK);
    let flown = fly(&mut battle, 900);
    assert!(!flown.chased, "a jet sent to circle turned to attack");
    assert!(
        flown.fired,
        "its self-guided missile never fired from the circle"
    );
    assert!(
        battle.unit(UnitId(0)).unwrap().orders.is_empty(),
        "the jet never reached its circle"
    );
}

#[test]
fn a_fired_on_jet_with_no_order_keeps_circling() {
    let mut battle = missile_jet_battle(Some(5));
    assert!(
        !fly(&mut battle, 600).chased,
        "an unordered jet turned on its attacker"
    );
}

#[test]
fn an_ordered_attack_still_flies_runs() {
    let mut battle = missile_jet_battle(None);
    for _ in 0..30 {
        battle.step();
    }
    let tank = battle
        .observe(Side::Blue)
        .identified
        .first()
        .map(|t| t.id)
        .expect("blue sees the tank");
    order(
        &mut battle,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank },
        },
    );
    assert!(
        fly(&mut battle, 900).chased,
        "an ordered attack never flew a run"
    );
}

#[test]
fn a_moved_jet_fired_on_keeps_circling() {
    let mut battle = missile_jet_battle(Some(5));
    move_to(&mut battle, [600.0, 400.0]);
    assert!(
        !fly(&mut battle, 600).chased,
        "a moved jet turned on its attacker"
    );
}

#[test]
fn an_attack_move_still_picks_up_an_enemy_on_its_way() {
    let mut battle = missile_jet_battle(None);
    attack_move_to(&mut battle, [1_500.0, 400.0]);
    assert!(
        fly(&mut battle, 900).chased,
        "an attack-move passed its enemy by"
    );
}

#[test]
fn an_arrived_attack_move_goes_after_an_attacker() {
    const GOAL: [f64; 2] = [600.0, 400.0];
    let mut battle = missile_jet_battle(Some(450));
    attack_move_to(&mut battle, GOAL);
    let before = fly(&mut battle, 440);
    assert!(!before.chased, "it attacked before anything was known");
    assert!(
        hunting_round(&battle, GOAL),
        "the attack-move ended or never arrived"
    );
    assert!(
        fly(&mut battle, 600).chased,
        "an arrived attack-move let its attacker be"
    );
}

#[test]
fn an_arrived_attack_move_goes_after_an_enemy_that_appears_near_its_goal() {
    const GOAL: [f64; 2] = [700.0, 400.0];
    let mut rules = narrow_field_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        // Mounts merge by id: the missile takes the place of its gun.
        json!({ "mounts": [{
            "id": "gun", "name": "AGM", "weapons": ["air_ground_missile"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
        }] }),
    );
    // The tank starts beyond the recon's sight and drives in past it.
    let setup = serde_json::from_value(json!({
        "map": { "size": [2400, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 400], "yaw": 0.0, "engagement": "fire_at_will" },
            { "side": "red", "kind": "test_tank", "position": [1700, 400], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_recon", "position": [800, 300], "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    attack_move_to(&mut battle, GOAL);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [900.0, 400.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    });
    assert_eq!(ack.error, None);
    let mut seen_at = None;
    for _ in 0..6_000 {
        let flown = fly(&mut battle, 1);
        let seen = !battle.observe(Side::Blue).identified.is_empty();
        if !seen {
            assert!(!flown.chased, "it attacked before the tank was seen");
            assert!(seen_at.is_none(), "the tank dropped out of sight");
            continue;
        }
        let tick = *seen_at.get_or_insert(battle.tick());
        if flown.chased {
            assert!(
                tick > 300,
                "the tank was seen before the jet had arrived at tick {tick}"
            );
            return;
        }
    }
    panic!("an arrived attack-move let a seen tank near its goal be (seen at {seen_at:?})");
}

/// A purchase arrives on a move to its destination, never an attack-move:
/// the jet's order ends on its circle there, and it holds.
#[test]
fn a_bought_jet_circles_its_destination_on_a_move() {
    let mut rules = fixtures::test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "roster": {"factions":["us","eastern"],"category":"veh","family_name":"Test jet","variant":"Test"} }),
    );
    let mut battle = Battle::new(&crate::skirmish::match_on(rules), 6);
    let ack = battle.accept(crate::common::command(
        Side::Blue,
        1,
        Order::purchase("test_jet", [400.0, 300.0]),
    ));
    assert_eq!(ack.error, None);
    battle.accept(crate::common::command(Side::Blue, 2, Order::Ready));
    battle.accept(crate::common::command(Side::Red, 1, Order::Ready));
    let mut moving = false;
    for _ in 0..(90 * battle.rules().tick_hz) {
        battle.step();
        let Some(jet) = battle.unit(UnitId(0)) else {
            continue;
        };
        match jet.orders.front() {
            Some(sim::units::UnitOrder::Move(m)) => {
                assert_eq!([m.destination.x, m.destination.y], [400.0, 300.0]);
                moving = true;
            }
            None if moving => return,
            other => panic!("a bought jet flies {other:?}"),
        }
    }
    panic!("the bought jet never reached its destination (moving: {moving})");
}
