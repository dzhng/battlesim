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
    battle_with(forces, zones, chapters, |_| {}, |_| {})
}

/// `battle`, with the scenario document and the test rules patched first.
fn battle_with(
    forces: Value,
    zones: Value,
    chapters: Value,
    patch_document: impl FnOnce(&mut Value),
    patch: impl FnOnce(&mut Value),
) -> Battle {
    let mut doc = document();
    patch_document(&mut doc);
    for (name, force) in forces.as_object().unwrap() {
        doc["forces"][name] = force.clone();
    }
    for (name, zone) in zones.as_object().unwrap() {
        doc["zones"][name] = zone.clone();
    }
    let mut chapters = chapters;
    chapters
        .as_array_mut()
        .unwrap()
        .push(crate::director::closing());
    doc["director"] = json!({ "chapters": chapters });
    // The smoke tank, the supply truck and the helicopter join the roster
    // for the facts about smoke, supply and the air.
    let rules = rules_with(|fixture| {
        patch(fixture);
        for (kind, variant) in [
            ("test_smoke_tank", "Smoke"),
            ("test_supply", "Supply"),
            ("test_heli", "Heli"),
        ] {
            sim::fixtures::patch_catalog(
                fixture,
                "units",
                kind,
                json!({"roster":{"factions":["us"],"category":"sup","family_name":"Test","variant":variant}}),
            );
        }
    });
    let mut b = Battle::new(&resolve(doc, rules).unwrap(), 9);
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

#[test]
fn smoke_popped_a_reverse_and_an_attack_on_a_named_unit_are_facts() {
    // A smoke tank faces red's tank (forces number by name: "smoker", "tank").
    let mut b = battle(
        json!({"smoker": {"side":"blue","owner":"blue-1","kind":"test_smoke_tank","position":[150.0,150.0],
                          "yaw":0.6,"engagement":"return_fire_only"},
               "tank": {"side":"red","owner":"red-1","kind":"test_tank","position":[260.0,180.0],
                        "yaw":3.0,"engagement":"return_fire_only"}}),
        json!({}),
        json!([{"id":"one","beats":[
            {"id":"smoked","when":{"units":{"who":{"force":"smoker"},"are":"popped_smoke"}}},
            {"id":"backing","when":{"units":{"who":{"force":"smoker"},"are":"reversing"}}},
            {"id":"targeted","when":{"units":{"who":{"force":"smoker"},"are":{"ordered":{"attack":{"force":"tank"}}}}}}
        ]}]),
    );
    b.step();
    assert_eq!(fired(&b, "smoked"), None);
    blue(
        &mut b,
        Order::PopSmoke {
            units: vec![UnitId(0)],
        },
    );
    until(&mut b, "smoked", 2);
    blue(
        &mut b,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 2,
            goal: [110.0, 125.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Reverse,
            facing: None,
        },
    );
    until(&mut b, "backing", 10);
    // The tank, by the handle blue identifies it by.
    let tank = b.observe(Side::Blue).identified[0].id;
    blue(
        &mut b,
        Order::Attack {
            units: vec![UnitId(0)],
            target: contract::command::TargetRef::Identified { id: tank },
        },
    );
    until(&mut b, "targeted", 2);
}

#[test]
fn a_recovery_beat_answers_a_loss_and_is_not_waited_for() {
    let chapters = |rifles_hold: &str| {
        (
            json!({"rifles": {"side":"blue","owner":"blue-1","kind":"test_rifle","position":[230.0,170.0],
                              "engagement": rifles_hold}}),
            json!([
                {"id":"one","beats":[
                    {"id":"started","when":{"phase":"active"}},
                    {"id":"replace","recovery":true,"when":{"destroyed":{"force":"rifles"}},
                     "then":[{"grant":{"player":"blue-1","credits":{"exactly":500}}}]}
                ]},
                {"id":"two","beats":[{"id":"next","when":"chapter_opens"}]}
            ]),
        )
    };
    // Nothing is lost: the next chapter opens without the recovery beat.
    let (forces, director) = chapters("return_fire_only");
    let mut quiet = battle(forces, json!({}), director);
    until(&mut quiet, "next", 5);
    assert_eq!(fired(&quiet, "replace"), None);
    // The rifles fire on red's tank beside them and die. A recovery beat
    // answers only while its chapter is open, so this chapter waits on the
    // loss itself.
    let (forces, mut director) = chapters("fire_at_will");
    director[0]["beats"][0] = json!({"id":"started","when":{"destroyed":{"force":"rifles"}}});
    let mut lost = battle(forces, json!({}), director);
    let wallet = |b: &Battle| b.observe(Side::Blue).skirmish.as_ref().unwrap().players[0].credits;
    let before = wallet(&lost);
    until(&mut lost, "replace", 120);
    assert!(
        wallet(&lost) >= before + 500.0,
        "the loss is answered with credits"
    );
}

#[test]
fn a_deployed_truck_services_a_worn_squad_and_fire_on_ground_is_ordered() {
    // Forces number by name: the rifles 0, red's tank 1 (holding fire), the
    // truck 2.
    let mut b = battle(
        json!({"rifles": {"side":"blue","owner":"blue-1","kind":"test_rifle","position":[100.0,212.0],
                          "engagement":"return_fire_only","condition":{"casualties":3}},
               "truck": {"side":"blue","owner":"blue-1","kind":"test_supply","position":[100.0,200.0],"stock":400},
               "tank": {"side":"red","owner":"red-1","kind":"test_tank","position":[380.0,280.0],
                        "yaw":3.0,"engagement":"return_fire_only"}}),
        json!({"far": {"center":[200.0,250.0],"radius_m":20.0}}),
        json!([{"id":"one","beats":[
            {"id":"set_up","when":{"units":{"who":{"force":"truck"},"are":"deployed"}}},
            {"id":"served","when":{"units":{"who":{"force":"rifles"},"are":"serviced"}}},
            {"id":"shelling","when":{"units":{"who":{"force":"rifles"},"are":{"ordered":{"attack_ground":"far"}}}}}
        ]}]),
    );
    blue(
        &mut b,
        Order::SetDeployment {
            units: vec![UnitId(2)],
            deployed: true,
        },
    );
    until(&mut b, "set_up", 60);
    until(&mut b, "served", 60);
    blue(
        &mut b,
        Order::Attack {
            units: vec![UnitId(0)],
            target: contract::command::TargetRef::Ground {
                point: [200.0, 250.0, 0.0],
            },
        },
    );
    until(&mut b, "shelling", 2);
}

#[test]
fn a_top_up_grant_adds_only_the_shortfall() {
    // The wallet after a grant, from an opening wallet of `opening` credits
    // (the scenario's own when none).
    let grant = |credits: Value, opening: Option<u64>| {
        let mut b = battle_with(
            json!({}),
            json!({}),
            json!([{"id":"one","beats":[{"id":"grant","when":"chapter_opens",
                "then":[{"grant":{"player":"blue-1","credits":credits}}]}]}]),
            |doc| {
                if let Some(opening) = opening {
                    doc["opening_credits"]["blue-1"] = json!({ "exactly": opening });
                }
            },
            |_| {},
        );
        let wallet =
            |b: &Battle| b.observe(Side::Blue).skirmish.as_ref().unwrap().players[0].credits;
        let before = wallet(&b);
        until(&mut b, "grant", 1);
        (before, wallet(&b))
    };
    let rifle = || json!([{"variant":"test_rifle"}]);
    // The scenario's opening wallet already covers a rifle squad.
    let (before, covered) = grant(json!({ "covering": rifle() }), None);
    assert!(covered > before, "covering adds the price");
    let (before, topped) = grant(json!({ "to_cover": rifle() }), None);
    assert_eq!(
        topped, before,
        "a wallet that already covers it gets nothing"
    );
    // An empty wallet is topped up by the whole price.
    let (_, from_empty) = grant(json!({ "covering": rifle() }), Some(0));
    let (empty, topped) = grant(json!({ "to_cover": rifle() }), Some(0));
    assert_eq!(empty, 0.0);
    assert_eq!(topped, from_empty, "a short wallet gets the shortfall");
}

#[test]
fn a_unit_withdrawn_is_not_destroyed() {
    // The player sends the rifles home; a loss would be answered, and the
    // chapter stays open on a beat that never fires.
    let mut b = battle(
        json!({"rifles": {"side":"blue","owner":"blue-1","kind":"test_rifle","position":[60.0,60.0],
                          "engagement":"return_fire_only"},
               "tank": {"side":"red","owner":"red-1","kind":"test_tank","position":[260.0,180.0],
                        "yaw":3.0,"engagement":"return_fire_only"}}),
        json!({}),
        json!([{"id":"one","beats":[
            {"id":"never","when":{"objective":{"id":"west","is":{"owned_by":"red"}}}},
            {"id":"lost","recovery":true,"when":{"destroyed":{"force":"rifles"}}}
        ]}]),
    );
    let rifles = |b: &Battle| b.observe(Side::Blue).own.iter().any(|u| u.id == UnitId(0));
    blue(
        &mut b,
        Order::Refund {
            units: vec![UnitId(0)],
        },
    );
    let limit = b.tick() + u64::from(120 * b.rules().tick_hz);
    while rifles(&b) {
        assert!(b.tick() < limit, "the rifles never left");
        b.step();
    }
    for _ in 0..b.rules().tick_hz {
        b.step();
    }
    assert_eq!(fired(&b, "lost"), None, "leaving the battle is no loss");
}

#[test]
fn a_radar_contact_is_the_finding_sides_alone() {
    // Blue's truck carries a radar receiver; red's tank a radar that emits.
    let mut b = battle_with(
        json!({"truck": {"side":"blue","owner":"blue-1","kind":"test_supply","position":[60.0,60.0]}}),
        json!({}),
        json!([{"id":"one","beats":[
            {"id":"blue_finds","when":{"radar_contact":{"side":"blue"}}},
            {"id":"red_finds","recovery":true,"when":{"radar_contact":{"side":"red"}}}
        ]}]),
        |_| {},
        |fixture| {
            sim::fixtures::patch_catalog(
                fixture,
                "units",
                "test_supply",
                // Short-sighted, so it finds the tank by radar alone.
                json!({"capabilities":{"radar_receiver":{"range_m":2000.0}},
                       "sensors":{"ground_m":30.0,"low_air_m":30.0,"high_air_m":30.0}}),
            );
            sim::fixtures::patch_catalog(
                fixture,
                "units",
                "test_tank",
                json!({"capabilities":{"radar_emitter":true}}),
            );
        },
    );
    until(&mut b, "blue_finds", 5);
    for _ in 0..40 {
        b.step();
    }
    assert_eq!(
        fired(&b, "red_finds"),
        None,
        "red has no receiver, so no radar contact"
    );
}

#[test]
fn a_purchase_is_pending_then_arrives_and_a_helicopter_flies() {
    let mut b = battle(
        json!({"heli": {"side":"blue","owner":"blue-1","kind":"test_heli","position":[80.0,80.0]}}),
        json!({}),
        json!([{"id":"one","beats":[
            {"id":"on_its_way","when":{"pending":{"player":"blue-1","purchase":{"variant":"test_rifle","carrier":"test_apc"}}}},
            {"id":"here","when":{"all":[{"fired":{"beat":"on_its_way"}},
                {"units":{"who":{"bought":{"player":"blue-1","variant":"test_apc"}},"are":"arrived"}}]}},
            {"id":"up","when":{"units":{"who":{"force":"heli"},"are":"airborne"}}}
        ]}]),
    );
    blue(
        &mut b,
        Order::ConfirmPurchase {
            variant: "test_rifle".into(),
            carrier: Some("test_apc".into()),
            destination: [60.0, 60.0],
        },
    );
    until(&mut b, "on_its_way", 2);
    until(&mut b, "here", 60);
    blue(
        &mut b,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [150.0, 100.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    until(&mut b, "up", 30);
}

#[test]
fn a_beat_answers_the_match_result_on_the_tick_it_is_decided() {
    // A blue squad on each of the three objectives, red's tank holding
    // fire: total control ends the match, and nothing ticks after.
    let mut doc = document();
    for (name, at) in [
        ("a", [170.0, 120.0]),
        ("b", [60.0, 160.0]),
        ("c", [280.0, 60.0]),
    ] {
        doc["forces"][name] =
            json!({"side":"blue","owner":"blue-1","kind":"test_rifle","position":at});
    }
    doc["forces"]["tank"]["engagement"] = json!("return_fire_only");
    doc["director"] = json!({"chapters":[crate::director::closing()]});
    let mut b = Battle::new(&resolve(doc, rules_with(|_| {})).unwrap(), 9);
    blue(&mut b, Order::Ready);
    let limit = b.tick() + 600 * u64::from(b.rules().tick_hz);
    while !b.finished() {
        assert!(b.tick() < limit, "blue takes total control");
        b.step();
    }
    assert_eq!(
        fired(&b, "won"),
        Some(b.tick()),
        "the beat answers the deciding tick"
    );
}

#[test]
fn a_contact_meets_the_zone_it_falls_in_and_any_holds_on_one_of_its_triggers() {
    // Blue's short-sighted truck finds red's emitting tank (at 260, 180) by
    // radar alone: a contact near the tank, never an identification.
    let mut b = battle_with(
        json!({"truck": {"side":"blue","owner":"blue-1","kind":"test_supply","position":[60.0,60.0]}}),
        json!({"at_tank": {"center":[260.0,180.0],"radius_m":30.0},
               "far_corner": {"center":[40.0,280.0],"radius_m":10.0}}),
        json!([{"id":"one","beats":[
            {"id":"at_tank","when":{"contact_in":{"side":"blue","zone":"at_tank"}}},
            {"id":"either","when":{"any":[
                {"contact_in":{"side":"blue","zone":"far_corner"}},
                {"contact_in":{"side":"blue","zone":"at_tank"}}]}},
            {"id":"elsewhere","recovery":true,"when":{"contact_in":{"side":"blue","zone":"far_corner"}}},
            {"id":"seen","recovery":true,"when":{"identifies":{"side":"blue","who":{"force":"tank"}}}}
        ]}]),
        |_| {},
        |fixture| {
            sim::fixtures::patch_catalog(
                fixture,
                "units",
                "test_supply",
                json!({"capabilities":{"radar_receiver":{"range_m":2000.0}},
                       "sensors":{"ground_m":30.0,"low_air_m":30.0,"high_air_m":30.0}}),
            );
            sim::fixtures::patch_catalog(
                fixture,
                "units",
                "test_tank",
                json!({"capabilities":{"radar_emitter":true}}),
            );
        },
    );
    until(&mut b, "at_tank", 5);
    until(&mut b, "either", 1);
    assert_eq!(
        fired(&b, "elsewhere"),
        None,
        "no contact meets the far corner"
    );
    assert_eq!(
        fired(&b, "seen"),
        None,
        "a radar contact identifies nothing"
    );
}

#[test]
fn a_side_identifies_the_unit_its_eyes_find() {
    // Blue's rifles stand in plain view of red's tank; nobody opens fire.
    let mut b = battle(
        json!({"rifles": {"side":"blue","owner":"blue-1","kind":"test_rifle","position":[150.0,150.0],
                          "engagement":"return_fire_only"},
               "tank": {"side":"red","owner":"red-1","kind":"test_tank","position":[260.0,180.0],
                        "yaw":3.0,"engagement":"return_fire_only"}}),
        json!({}),
        json!([{"id":"one","beats":[
            {"id":"seen","when":{"identifies":{"side":"blue","who":{"force":"tank"}}}},
            {"id":"seen_back","when":{"identifies":{"side":"red","who":{"force":"rifles"}}}}
        ]}]),
    );
    until(&mut b, "seen", 5);
    until(&mut b, "seen_back", 5);
}
