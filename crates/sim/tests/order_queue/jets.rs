//! The order queue for winged aircraft: a jet's volley is a run in which it
//! fired, or a weapon cycle in a dogfight, and its attack ends when its side
//! loses the target.
use super::*;

/// A jet given an attack on an enemy jet behind a move flies the move first:
/// the dogfight starts on the attack's turn, not when it is queued.
#[test]
fn a_queued_attack_on_a_jet_starts_no_dogfight_before_its_turn() {
    use crate::attack_runs::{run_field, run_rules, RUN_START};
    let mut b = run_field(
        run_rules(&["air_gun"]),
        json!([
            { "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 },
            { "side": "red", "kind": "test_jet", "position": [RUN_START[0] + 600.0, RUN_START[1]], "yaw": std::f64::consts::PI },
        ]),
    );
    let mut c = Commander::new();
    let enemy = seen(&mut b);
    c.ok(
        &mut b,
        Side::Blue,
        drive([RUN_START[0] + 300.0, RUN_START[1] - 300.0]),
    );
    assert_eq!(c.send(&mut b, Side::Blue, attack(0, enemy), true), None);
    let dogfight = |b: &Battle| b.unit(UnitId(0)).unwrap().air.unwrap().dogfight_target;
    until(&mut b, ticks(120.0), "the attack's turn", |b| {
        if !attacking(b, 0) {
            assert_eq!(dogfight(b), None, "dogfighting before its turn");
        }
        attacking(b, 0)
    });
    until(&mut b, ticks(60.0), "the dogfight on its turn", |b| {
        dogfight(b).is_some()
    });
}

/// A test jet carrying `weapons` on the run field, attacking `targets` on the
/// ground in turn (each after the first shift-queued), then shift-moving.
/// Returns the runs it flew that fired, before it turned to the move.
fn jet_runs(weapons: &[&str], targets: &[[f64; 2]], window_s: f64) -> (Battle, u32) {
    use crate::attack_runs::{run_field, run_rules, RUN_START};
    use sim::air::pass::PassPhase;
    let mut b = run_field(
        run_rules(weapons),
        json!([{ "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 }]),
    );
    let mut c = Commander::new();
    for (k, &p) in targets.iter().enumerate() {
        assert_eq!(c.send(&mut b, Side::Blue, ground_at(0, p), k > 0), None);
    }
    let away = [RUN_START[0], RUN_START[1] + 1200.0];
    assert_eq!(c.send(&mut b, Side::Blue, drive(away), true), None);
    b.step();
    let mut runs = 0;
    let (mut diving, mut fired) = (false, false);
    for _ in 0..ticks(window_s) {
        let before = b
            .unit(UnitId(0))
            .unwrap()
            .mounts
            .iter()
            .map(|m| m.shots)
            .sum::<u32>();
        b.step();
        let jet = b.unit(UnitId(0)).unwrap();
        let now = jet
            .air
            .unwrap()
            .pass
            .is_some_and(|p| p.phase == PassPhase::Dive);
        fired |= jet.mounts.iter().map(|m| m.shots).sum::<u32>() > before;
        if diving && !now && fired {
            runs += 1;
        }
        if !now {
            fired = false;
        }
        diving = now;
        if !attacking(&b, 0) {
            break;
        }
    }
    (b, runs)
}

/// The same point shift-attacked twice, then a move: two bombing runs, then
/// the move.
#[test]
fn a_jet_queued_twice_on_one_target_flies_two_runs_then_moves() {
    let aim = [
        crate::attack_runs::RUN_START[0] + 1500.0,
        crate::attack_runs::RUN_START[1],
    ];
    let (b, runs) = jet_runs(&["air_bomb"], &[aim, aim], 240.0);
    assert!(!attacking(&b, 0), "still attacking after the window");
    assert_eq!(runs, 2);
}

/// A jet with nothing to fly a run at the ground with, told to attack it with
/// a move queued, hands off at once.
#[test]
fn a_jet_with_no_ground_weapon_hands_off_at_once() {
    let aim = [
        crate::attack_runs::RUN_START[0] + 1500.0,
        crate::attack_runs::RUN_START[1],
    ];
    let (b, runs) = jet_runs(&["air_missile"], &[aim], 2.0);
    assert!(!attacking(&b, 0));
    assert_eq!(runs, 0);
}

/// Near a corner of the field, where the edge guard breaks runs off, a jet
/// still flies a run that fires and hands off.
#[test]
fn a_jet_near_a_corner_still_hands_off() {
    let aim = [250.0, 250.0];
    let (b, runs) = jet_runs(&["air_bomb"], &[aim], 360.0);
    assert!(!attacking(&b, 0), "never handed off near the corner");
    assert_eq!(runs, 1);
}

/// The acceptance queue for a jet, as for the ground unit: each attack is a
/// fired run, every move is flown, and it goes home.
#[test]
fn a_jet_works_through_a_queue_of_attacks_moves_and_home() {
    let mut s = Skirmish::new("test_jet", "air", [2400.0, 1800.0]);
    let jet = s.unit;
    s.ok(Side::Blue, move_to(vec![jet], [1200.0, 900.0]), false);
    let a = s.seen_near([990.0, 1260.0]);
    let other = s.seen_near([1410.0, 1260.0]);
    let attack = |id| Order::Attack {
        units: vec![jet],
        target: TargetRef::Identified { id },
    };
    s.ok(Side::Blue, attack(a), false);
    s.ok(Side::Blue, move_to(vec![jet], [600.0, 600.0]), true);
    s.ok(Side::Blue, attack(a), true);
    s.ok(Side::Blue, move_to(vec![jet], [1800.0, 600.0]), true);
    s.ok(Side::Blue, attack(other), true);
    s.ok(Side::Blue, Order::Refund { units: vec![jet] }, true);
    s.b.step();
    let seen = follow(&mut s.b, jet, ticks(600.0));
    let kinds: Vec<_> = seen.iter().map(|s| s.kind).collect();
    assert_eq!(
        kinds,
        ["attack", "move", "attack", "move", "attack", "home"],
        "{seen:?}"
    );
    assert!(
        seen[0].alive_at_end,
        "the first attack fought on until its target died: {seen:?}"
    );
    for w in (seen.windows(2)).filter(|w| w[0].kind == "attack" && w[0].alive_at_start) {
        assert!(
            w[1].shots > w[0].shots,
            "an attack handed off without firing: {seen:?}"
        );
    }
}

/// A jet attacking a squad only its side's jeep can see ends the attack
/// once the jeep drives off and the side loses the squad: it circles there
/// rather than attacking nothing for ever.
#[test]
fn a_jets_attack_ends_when_its_side_loses_the_target() {
    use crate::attack_runs::{run_field, run_rules, RUN_START};
    let [x, y] = RUN_START;
    let mut b = run_field(
        run_rules(&["air_gun"]),
        json!([
            { "side": "blue", "kind": "test_jet", "position": [x, y - 1500.0], "yaw": 0.0 },
            { "side": "blue", "kind": "test_jeep", "position": [x, y], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_smoke_squad", "position": [x + 150.0, y], "engagement": "return_fire_only" },
        ]),
    );
    let mut c = Commander::new();
    let squad = seen(&mut b);
    c.ok(&mut b, Side::Blue, attack(0, squad));
    let off = Order::Move {
        units: vec![UnitId(1)],
        gesture: 2,
        goal: [x, y + 1800.0],
        route: contract::command::RoutePolicy::Shortest,
        direction: contract::command::MoveDirection::Forward,
        facing: None,
    };
    c.ok(&mut b, Side::Blue, off);
    until(&mut b, ticks(120.0), "the squad lost", |b| {
        b.observe(Side::Blue).identified.is_empty()
    });
    until(&mut b, ticks(10.0), "the attack over", |b| !attacking(b, 0));
    assert!(
        b.unit(UnitId(2)).unwrap().alive(),
        "the squad was killed, not lost"
    );
}

/// A jet carrying a gun and air-to-air missiles, told to attack an enemy
/// aircraft (the only enemy on the field) with a move queued behind, fires
/// at it and hands off while the enemy still flies, then flies the move. The
/// two cases are the two volleys a jet fires at an aircraft: a weapon cycle
/// at an enemy jet (a dogfight), a run at a helicopter. The enemy is made too
/// sturdy for one volley to bring down, so only the hand-off can end the
/// attack.
fn jet_attacks_aircraft_then_moves(enemy: &str) {
    use crate::attack_runs::{run_field, run_rules, RUN_START};
    let [x, y] = RUN_START;
    let mut rules = run_rules(&["air_gun", "air_missile"]);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        enemy,
        json!({ "body": { "hull": { "hp": 100000 } } }),
    );
    let mut b = run_field(
        rules,
        json!([
            { "side": "blue", "kind": "test_jet", "position": [x, y], "yaw": 0.0 },
            { "side": "red", "kind": enemy, "position": [x + 700.0, y], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
        ]),
    );
    let mut c = Commander::new();
    let target = seen(&mut b);
    c.ok(&mut b, Side::Blue, attack(0, target));
    let away = [x, y + 1200.0];
    assert_eq!(c.send(&mut b, Side::Blue, drive(away), true), None);
    // The commands apply on the next tick.
    b.step();
    let shots = |b: &Battle| -> u32 {
        b.unit(UnitId(0))
            .unwrap()
            .mounts
            .iter()
            .map(|m| m.shots)
            .sum()
    };
    until(&mut b, ticks(180.0), "the attack handed off", |b| {
        !attacking(b, 0)
    });
    assert!(shots(&b) > 0, "handed off without firing at the {enemy}");
    assert!(
        b.unit(UnitId(1)).unwrap().alive(),
        "the {enemy} died: no hand-off shown"
    );
    let front = &b.unit(UnitId(0)).unwrap().orders.front().cloned();
    assert!(matches!(front, Some(UnitOrder::Move(_))), "{front:?}");
    until(&mut b, ticks(60.0), "the queued move flown", |b| {
        near(b, 0, away, 300.0)
    });
}

#[test]
fn a_jet_fires_at_an_enemy_jet_then_flies_the_queued_move() {
    jet_attacks_aircraft_then_moves("test_jet");
}

#[test]
fn a_jet_fires_at_a_helicopter_then_flies_the_queued_move() {
    jet_attacks_aircraft_then_moves("test_heli");
}
