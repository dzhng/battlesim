//! What a director reads and orders, on test units: a squad aboard, unloaded
//! and garrisoned; suppressed; attack-moving; an objective taken and
//! contested. Each fact is read however it came about.

use crate::scenarios::{document, resolve, rules_with};
use contract::command::Order;
use contract::ids::{Side, UnitId};
use contract::players::ControllerKind;
use serde_json::{json, Value};
use sim::battle::Battle;

/// The sample scenario with `forces` added and `chapters` as its director.
fn battle(forces: Value, zones: Value, chapters: Value) -> Battle {
    let mut doc = document();
    for (name, force) in forces.as_object().unwrap() {
        doc["forces"][name] = force.clone();
    }
    for (name, zone) in zones.as_object().unwrap() {
        doc["zones"][name] = zone.clone();
    }
    doc["director"] = json!({ "chapters": chapters });
    let mut b = Battle::new(&resolve(doc, rules_with(|_| {})).unwrap(), 9);
    blue(&mut b, Order::Ready);
    b
}

fn blue(b: &mut Battle, order: Order) {
    let ack = b.accept_order(Side::Blue, Some("blue-1".into()), order, false);
    assert_eq!(ack.error, None, "the player's order is taken");
}

fn fired(b: &Battle, beat: &str) -> Option<u64> {
    b.director().unwrap().fired_at(beat)
}

/// Steps until `beat` fires, at most `seconds` of battle.
fn until(b: &mut Battle, beat: &str, seconds: u32) -> u64 {
    let limit = b.tick() + u64::from(seconds * b.rules().tick_hz);
    while fired(b, beat).is_none() {
        assert!(b.tick() < limit, "{beat} never fired");
        b.step();
    }
    fired(b, beat).unwrap()
}

#[test]
fn a_directed_carrier_unloads_and_its_squad_occupies_the_nearest_building() {
    // Forces number by name: the carrier 0, the squad 1. The test map's one
    // building stands at (340, 110). Nobody opens fire.
    let mut b = battle(
        json!({
            "carrier": {"side":"blue","owner":"blue-1","kind":"test_apc","position":[290.0,100.0],
                        "engagement":"return_fire_only"},
            "squad": {"side":"blue","owner":"blue-1","kind":"test_rifle","position":[296.0,104.0],
                      "engagement":"return_fire_only"},
            "tank": {"side":"red","owner":"red-1","kind":"test_tank","position":[260.0,180.0],
                     "yaw":3.0,"engagement":"return_fire_only"}
        }),
        json!({"shed": {"center":[340.0,110.0],"radius_m":30.0},
               "set_down": {"center":[290.0,100.0],"radius_m":25.0}}),
        json!([{"id":"one","beats":[
            {"id":"aboard","when":{"units":{"who":{"force":"squad"},"are":{"inside":"carrier"}}}},
            {"id":"unload","when":{"fired":{"beat":"aboard","after_s":1}},
             "then":[{"order":{"units":{"force":"carrier"},"order":"unload"}}]},
            {"id":"down","when":{"all":[{"fired":{"beat":"unload"}},
                {"units":{"who":{"force":"squad"},"are":{"in":"set_down"}}}]},
             "then":[{"order":{"units":{"force":"squad"},"order":{"occupy":"shed"}}}]},
            {"id":"garrisoned","when":{"units":{"who":{"force":"squad"},"are":{"inside":"building"}}}}
        ]}]),
    );
    // The player boards it; the director takes the seat from then on.
    blue(
        &mut b,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    );
    until(&mut b, "aboard", 30);
    b.set_controllers(vec![("blue-1".into(), ControllerKind::DirectedAi)]);
    until(&mut b, "down", 30);
    until(&mut b, "garrisoned", 60);
    assert!(b.director().unwrap().refusals().is_empty());
}

#[test]
fn attack_moving_and_an_objective_taken_then_contested_are_facts() {
    // The squad 0 walks into the west objective; the red tank comes after.
    let mut b = battle(
        json!({"rifles": {"side":"blue","owner":"blue-1","kind":"test_rifle","position":[60.0,110.0]}}),
        json!({"west": {"center":[60.0,160.0],"radius_m":20.0}}),
        json!([{"id":"one","beats":[
            {"id":"moving","when":{"units":{"who":{"force":"rifles"},"are":"attack_moving"}}},
            {"id":"capturing","when":{"objective":{"id":"west","is":{"capturing":"blue"}}}},
            {"id":"taken","when":{"objective":{"id":"west","is":{"owned_by":"blue"}}},
             "then":[{"order":{"units":{"force":"tank"},"order":{"move_to":"west"}}}]},
            {"id":"contested","when":{"objective":{"id":"west","is":"contested"}}}
        ]}]),
    );
    blue(
        &mut b,
        Order::AttackMove {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [60.0, 160.0],
        },
    );
    let moving = until(&mut b, "moving", 2);
    let capturing = until(&mut b, "capturing", 60);
    let taken = until(&mut b, "taken", 120);
    assert!(moving < capturing && capturing < taken);
    until(&mut b, "contested", 120);
}

#[test]
fn a_squad_under_fire_is_suppressed() {
    // A tank firing at will on a squad in the open, close by.
    let mut b = battle(
        json!({"rifles": {"side":"blue","owner":"blue-1","kind":"test_rifle","position":[220.0,180.0],
                          "engagement":"return_fire_only"}}),
        json!({}),
        json!([{"id":"one","beats":[
            {"id":"suppressed","when":{"units":{"who":{"force":"rifles"},"are":{"suppression":"suppressed"}}}}
        ]}]),
    );
    until(&mut b, "suppressed", 60);
}
