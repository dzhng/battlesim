//! `attacks_only_on_order`: a unit that authors it fights only under an
//! attack or attack-move order. Sent somewhere, or left with no order, it
//! circles and holds fire, even at an enemy in plain sight or one shooting
//! at it; an explicit attack still runs as ever.
use crate::common::air_field::narrow_field_rules;
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;
use sim::fixtures;

/// A test jet carrying a guided ground missile (it needs no nose-on pass to
/// fire) 400 m from a red tank, firing at will; `only_on_order` authors the
/// capability. Nothing in high air sees the ground: with `fire_event` the
/// tank fires and the jet's side knows it only as that firing contact;
/// without, a blue recon squad beside the tank spots it for the jet.
fn missile_jet_battle(only_on_order: bool, fire_event: bool) -> Battle {
    let mut rules = narrow_field_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({
            "mounts": [{
                "id": "agm", "name": "AGM", "weapons": ["air_ground_missile"],
                "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [4, 0, 0]
            }],
            "capabilities": { "attacks_only_on_order": only_on_order }
        }),
    );
    let (events, spotter) = if fire_event {
        (json!([{ "tick": 5, "fire": { "unit": 1 } }]), json!([]))
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

fn move_over_the_tank(battle: &mut Battle) {
    order(
        battle,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [900.0, 400.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    );
}

/// Steps `ticks` and reports whether the jet fired or began a run.
fn engaged(battle: &mut Battle, ticks: u32) -> bool {
    for _ in 0..ticks {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        if jet.mounts[0].shots > 0 || jet.air.unwrap().pass.is_some() {
            return true;
        }
    }
    false
}

#[test]
fn a_jet_sent_over_a_seen_enemy_circles_without_attacking_it() {
    // Without the capability the same jet takes the shot: the scene can
    // engage, so the hold below is the capability's.
    let mut free = missile_jet_battle(false, false);
    move_over_the_tank(&mut free);
    assert!(engaged(&mut free, 900), "the scene never offers a shot");

    let mut held = missile_jet_battle(true, false);
    move_over_the_tank(&mut held);
    assert!(
        !engaged(&mut held, 900),
        "a jet sent to circle attacked on its own"
    );
    assert!(
        held.unit(UnitId(0)).unwrap().orders.is_empty(),
        "the jet never reached its circle"
    );
}

#[test]
fn a_fired_on_jet_with_no_order_does_not_answer() {
    let mut free = missile_jet_battle(false, true);
    assert!(engaged(&mut free, 600), "the scene never draws an answer");

    let mut held = missile_jet_battle(true, true);
    assert!(
        !engaged(&mut held, 600),
        "an unordered jet answered its attacker"
    );
}

#[test]
fn an_ordered_attack_still_fires() {
    let mut battle = missile_jet_battle(true, false);
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
    assert!(engaged(&mut battle, 900), "an ordered attack never fired");
}

#[test]
fn an_attack_move_still_picks_up_an_enemy_on_its_way() {
    let mut battle = missile_jet_battle(true, false);
    order(
        &mut battle,
        Order::AttackMove {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [1_500.0, 400.0],
        },
    );
    assert!(
        engaged(&mut battle, 900),
        "an attack-move passed its enemy by"
    );
}
