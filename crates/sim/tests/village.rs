//! The village encounter (slice 15): the authored scenario, the defender
//! policy on its own observation, the referee, and replay of both sides.
use contract::command::{CommandEnvelope, Engagement, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{EncounterResult, GarrisonPhase, ObservationFrame};
use contract::scenario::{ScenarioDefinition, UnitSetup};
use serde_json::json;
use sim::battle::Battle;
use sim::village::scripts::Plan;
use sim::village::{scenario, trial};

use crate::common;

/// Ordinary ids: blue 0–8 (the jeep last); red rifles 9–11, the AT team 12,
/// the tank 13, the jeep 14.
const RED_AT: u32 = 12;
const BLUE_TANKS: [u32; 2] = [4, 5];

fn setup(variant: &str) -> ScenarioDefinition {
    scenario(&common::village(), variant).unwrap()
}

fn hz() -> u64 {
    common::tick_hz() as u64
}

fn order(battle: &mut Battle, seq: u64, order: Order) {
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        seq,
        order,
        queued: false,
    });
    assert_eq!(ack.error, None);
}

fn push(goal: [f64; 2]) -> Order {
    Order::Move {
        units: BLUE_TANKS.map(UnitId).to_vec(),
        gesture: 1,
        goal,
        route: RoutePolicy::Shortest,
        direction: contract::command::MoveDirection::Forward,
        facing: None,
    }
}

/// Red's accepted orders so far, with the tick each applied at.
fn red_orders(battle: &Battle) -> Vec<(u64, Order)> {
    battle
        .replay()
        .accepted
        .into_iter()
        .filter(|(_, c)| c.side == Side::Red)
        .map(|(t, c)| (t, c.order))
        .collect()
}

#[test]
fn the_variants_differ_only_by_the_second_at_team() {
    let reds = |v: &str| -> Vec<(String, [f64; 2])> {
        setup(v)
            .units
            .iter()
            .filter(|u| u.side == Side::Red)
            .map(|u| (u.kind.clone(), u.position))
            .collect()
    };
    let (ordinary, crossfire) = (reds("ordinary"), reds("prepared_crossfire"));
    assert_eq!(ordinary.len() + 1, crossfire.len());
    assert!(!ordinary.contains(&("at".to_string(), [1120.0, 650.0])));
    assert!(crossfire.contains(&("at".to_string(), [1120.0, 650.0])));
    assert!(ordinary.iter().all(|u| crossfire.contains(u)));
    // AT teams start holding fire; everyone else fires at will.
    for u in setup("prepared_crossfire").units {
        let holding = u.engagement == Some(Engagement::ReturnFireOnly);
        assert_eq!(holding, u.side == Side::Red && u.kind == "at");
    }
    assert!(scenario(&common::village(), "no_such_variant").is_err());
}

#[test]
fn a_spawn_row_may_set_its_units_engagement() {
    let mut fixture = common::village();
    fixture["spawn"]["blue"][8] = serde_json::json!(["jeep", 125, 905, "return_fire_only"]);
    fixture["spawn"]["red"][3] = serde_json::json!(["at", 760, 886, "fire_at_will"]);
    let units = scenario(&fixture, "ordinary").unwrap().units;
    let engagement = |side: Side, kind: &str| {
        units
            .iter()
            .filter(|u| u.side == side && u.kind == kind)
            .map(|u| u.engagement)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        engagement(Side::Blue, "jeep"),
        [Some(Engagement::ReturnFireOnly)]
    );
    // The column overrides the side's default (red AT teams hold fire).
    assert_eq!(engagement(Side::Red, "at"), [Some(Engagement::FireAtWill)]);
    // Rows without it keep the defaults.
    assert_eq!(engagement(Side::Blue, "tank"), [None, None]);

    for bad in [
        serde_json::json!(["jeep", 125, 905, "hold_fire"]),
        serde_json::json!(["jeep", 125, 905, "return_fire_only", 1]),
        serde_json::json!(["jeep", 125]),
    ] {
        let mut fixture = common::village();
        fixture["spawn"]["blue"][8] = bad.clone();
        assert!(scenario(&fixture, "ordinary").is_err(), "{bad} loads");
    }
}

#[test]
fn the_defender_garrisons_the_three_buildings() {
    let mut battle = Battle::new(&setup("ordinary"), 1);
    for _ in 0..20 * hz() {
        battle.step();
    }
    let inside: Vec<u32> = battle
        .observe(Side::Red)
        .own
        .iter()
        .filter(|u| {
            u.garrison
                .as_ref()
                .is_some_and(|g| g.phase == GarrisonPhase::Inside)
        })
        .map(|u| u.id.0)
        .collect();
    assert_eq!(inside, vec![9, 10, 11]);
    // The garrison orders were ordinary commands, accepted once.
    let garrisons = red_orders(&battle)
        .iter()
        .filter(|(_, o)| matches!(o, Order::Garrison { .. }))
        .count();
    assert_eq!(garrisons, 3);
}

#[test]
fn the_at_team_attacks_only_once_its_own_optics_identify_a_tank() {
    let mut battle = Battle::new(&setup("ordinary"), 1);
    // The tanks drive to the forest edge, past the hidden AT team.
    order(&mut battle, 1, push([700.0, 870.0]));
    let mut sighted = None;
    for _ in 0..120 * hz() {
        battle.step();
        let frame = battle.observe(Side::Red);
        // The team may die in the fight it starts: the rule is judged while it lives.
        let Some(at) = frame.own.iter().find(|u| u.id.0 == RED_AT) else {
            break;
        };
        let seen = frame.identified.iter().any(|e| {
            e.kind == common::unit_kind("tank")
                && at.sees.contains(&e.id)
                && (e.position[0] - at.position[0]).hypot(e.position[1] - at.position[1]) <= 900.0
        });
        if seen && sighted.is_none() {
            sighted = Some(battle.tick());
        }
        if sighted.is_none() {
            assert_eq!(
                at.engagement,
                Engagement::ReturnFireOnly,
                "held fire until sighting"
            );
        }
    }
    let sighted = sighted.expect("the AT team's own optics identified a tank");
    let attacks: Vec<u64> = red_orders(&battle)
        .into_iter()
        .filter(|(_, o)| matches!(o, Order::Attack { units, .. } if units == &[UnitId(RED_AT)]))
        .map(|(t, _)| t)
        .collect();
    assert!(!attacks.is_empty(), "the AT team attacked");
    assert!(attacks[0] > sighted, "no attack before its own sighting");
    assert!(
        attacks[0] <= sighted + 2,
        "the attack follows the sighting at once"
    );
}

/// Red decides from red's observation only: a change red cannot observe
/// changes neither red's frames nor red's orders.
#[test]
fn hidden_blue_state_does_not_change_red_decisions() {
    let run = |hidden: bool| {
        let mut s = setup("ordinary");
        if hidden {
            // The supply truck's stock, far back west, is blue's alone to know.
            let truck: &mut UnitSetup = s.units.iter_mut().find(|u| u.kind == "supply").unwrap();
            truck.stock = Some(1);
        }
        let mut battle = Battle::new(&s, 3);
        order(&mut battle, 1, push([700.0, 870.0]));
        let mut frames: Vec<String> = Vec::new();
        for _ in 0..90 * hz() {
            battle.step();
            let f: &ObservationFrame = battle.observe(Side::Red);
            frames.push(serde_json::to_string(f).unwrap());
        }
        (frames, red_orders(&battle), battle.digest())
    };
    let (frames_a, orders_a, digest_a) = run(false);
    let (frames_b, orders_b, digest_b) = run(true);
    assert_ne!(digest_a, digest_b, "the hidden change is real");
    assert!(frames_a == frames_b, "red observes the same battle");
    assert!(!orders_a.is_empty());
    assert_eq!(
        serde_json::to_string(&orders_a).unwrap(),
        serde_json::to_string(&orders_b).unwrap()
    );
}

/// A replay applies both sides' accepted commands with the defender off:
/// every tick digest matches and no red order is issued twice.
#[test]
fn a_replay_matches_every_digest_without_rerunning_the_defender() {
    let s = setup("prepared_crossfire");
    let mut live = Battle::new(&s, 21);
    order(&mut live, 1, push([900.0, 800.0]));
    let mut digests = Vec::new();
    for _ in 0..60 * hz() {
        live.step();
        digests.push(live.digest());
    }
    let recorded = live.replay();
    assert!(recorded.accepted.iter().any(|(_, c)| c.side == Side::Red));
    let mut replayed = Battle::from_replay(&s, &recorded).unwrap();
    for (i, want) in digests.iter().enumerate() {
        replayed.step();
        assert_eq!(replayed.digest(), *want, "tick {}", i + 1);
    }
    assert_eq!(
        serde_json::to_string(&replayed.replay()).unwrap(),
        serde_json::to_string(&recorded).unwrap(),
        "the replay issued no commands of its own"
    );
    // The other variant's scenario refuses the file.
    assert!(Battle::from_replay(&setup("ordinary"), &recorded).is_err());
}

/// The referee on hand-placed units: an uncontested hold captures; a
/// defender in the zone contests; no attacking combat unit is defeat.
#[test]
fn the_referee_captures_contests_and_defeats() {
    let placed = |units: Vec<(Side, &str, [f64; 2])>| {
        let mut s = setup("ordinary");
        s.opponent = None;
        s.units = units
            .into_iter()
            .map(|(side, kind, position)| UnitSetup {
                side,
                kind: kind.to_string(),
                position,
                yaw: 0.0,
                engagement: Some(Engagement::ReturnFireOnly),
                condition: None,
                stock: None,
            })
            .collect();
        Battle::new(&s, 1)
    };
    let hold = 30 * hz();
    let far = [1500.0, 1500.0];
    let mut held = placed(vec![
        (Side::Blue, "rifle", [1000.0, 820.0]),
        (Side::Red, "rifle", far),
    ]);
    for _ in 1..hold {
        held.step();
    }
    let status = held.observe(Side::Blue).encounter.unwrap();
    assert_eq!(status.result, EncounterResult::Running);
    held.step();
    let status = held.observe(Side::Red).encounter.unwrap();
    assert_eq!(status.result, EncounterResult::Captured);
    assert_eq!(status.held_s, 30.0);

    let mut contested = placed(vec![
        (Side::Blue, "rifle", [1000.0, 820.0]),
        (Side::Red, "supply", [1010.0, 780.0]),
    ]);
    for _ in 0..hold + 10 {
        contested.step();
    }
    let status = contested.observe(Side::Blue).encounter.unwrap();
    assert_eq!(
        (status.result, status.held_s),
        (EncounterResult::Running, 0.0)
    );

    // A supply truck alone is not a combat force.
    let mut beaten = placed(vec![
        (Side::Blue, "supply", [1000.0, 820.0]),
        (Side::Red, "rifle", far),
    ]);
    beaten.step();
    assert_eq!(
        beaten.observe(Side::Blue).encounter.unwrap().result,
        EncounterResult::Defeated
    );
}

/// Whether a unit fights is its components, not its role: a supply truck
/// listed under a fighting role is still no combat force, and one that
/// carries a gun is.
#[test]
fn the_referee_counts_units_that_carry_weapons() {
    let alone = |patch: serde_json::Value| {
        let mut s = setup("ordinary");
        s.opponent = None;
        let mut rules = serde_json::to_value(&s.rules).unwrap();
        sim::fixtures::patch_catalog(&mut rules, "units", "supply", patch);
        s.rules = serde_json::from_value(rules).unwrap();
        s.units = [(Side::Blue, [1000.0, 820.0]), (Side::Red, [1500.0, 1500.0])]
            .into_iter()
            .map(|(side, position)| UnitSetup {
                side,
                kind: if side == Side::Blue {
                    "supply"
                } else {
                    "rifle"
                }
                .to_string(),
                position,
                yaw: 0.0,
                engagement: Some(Engagement::ReturnFireOnly),
                condition: None,
                stock: None,
            })
            .collect();
        let mut b = Battle::new(&s, 1);
        b.step();
        b.observe(Side::Blue).encounter.unwrap().result
    };
    let fighting_role = json!({ "roles": ["light_vehicle"] });
    assert_eq!(alone(fighting_role), EncounterResult::Defeated);
    let gun = json!({ "mounts": [{ "name": "HMG", "weapons": ["hmg"], "turret": true,
                                   "pivot_m": [0, 0, 2], "muzzle_m": [1, 0, 0] }] });
    assert_eq!(alone(gun), EncounterResult::Running);
}

/// A scripted trial is repeatable from its seed (the full ten-seed
/// comparison is `cargo run -p sim --release --example village_report`).
#[test]
fn a_scripted_trial_repeats_from_its_seed() {
    let fixture = common::village();
    let plan = Plan::ScoutSuppressFlank;
    let (a, b) = std::thread::scope(|s| {
        let a = s.spawn(|| trial(&fixture, "ordinary", plan, 5, 120.0));
        let b = s.spawn(|| trial(&fixture, "ordinary", plan, 5, 120.0));
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_eq!(a, b);
    assert_eq!(a.result, EncounterResult::Running);
    assert_eq!(a.rejected, 0, "every script order is a legal command");
}

/// Battle-look slice 26: smoke is presentation only. A burning wreck's
/// smoke and fire live in `presentation.effects`, which the simulation never
/// reads, so no smoke can hide anything: the battle, its wrecks and what each
/// side sees, is the same whatever the smoke looks like, or with none.
#[test]
fn smoke_is_presentation_only() {
    let run = |fixture: &serde_json::Value| {
        let mut battle = Battle::new(&scenario(fixture, "ordinary").unwrap(), 1);
        order(&mut battle, 1, push([700.0, 870.0]));
        for _ in 0..120 * hz() {
            battle.step();
        }
        let wrecks = [Side::Blue, Side::Red].map(|side| {
            battle
                .observe(side)
                .known_props
                .iter()
                .filter(|p| {
                    [common::kind("tank_wreck"), common::kind("supply_wreck")].contains(&p.kind)
                })
                .count()
        });
        (battle.digest(), wrecks)
    };
    let plain = common::village();
    let mut thick = plain.clone();
    let wreck = &mut thick["presentation"]["effects"]["smoke"]["wreck"];
    wreck["smoke"]["opacity"] = 1.0.into();
    wreck["smoke"]["size_m"] = serde_json::json!([20.0, 80.0]);
    wreck["burn_s"] = 100000.0.into();
    let mut none = plain.clone();
    none["presentation"]["effects"]
        .as_object_mut()
        .unwrap()
        .remove("smoke");
    let [plain, thick, none] = std::thread::scope(|s| {
        [&plain, &thick, &none]
            .map(|f| s.spawn(move || run(f)))
            .map(|h| h.join().unwrap())
    });
    assert!(plain.1.iter().any(|&n| n > 0), "the battle leaves a wreck");
    assert_eq!(thick, plain);
    assert_eq!(none, plain);
}
