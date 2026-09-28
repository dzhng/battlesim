//! Destroyable props (Q17, slice 34c): every kind with integrity wears down
//! under structural fire, by a direct round scaled by its armour and by a
//! burst's blast scaled by distance, and is destroyed into its row's state.
//! Driven through real battles; bursts are the scenario emitter's.
use std::collections::BTreeMap;

use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::map::{MoverClass, PropKind};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, Struck};
use sim::math::v2;

use crate::common;

fn rules() -> Value {
    common::village()
}

fn row(kind: &str) -> Value {
    rules()["props"][kind].clone()
}

fn he() -> (f64, f64) {
    let w = &rules()["weapons"]["tank_he"];
    (
        w["structural_damage"].as_f64().unwrap(),
        w["blast_radius_m"].as_f64().unwrap(),
    )
}

fn prop(kind: &str, center: [f64; 2], half: [f64; 3]) -> Value {
    json!({ "kind": kind, "center": center, "yaw": 0, "half_extents": half })
}

fn burst(tick: u64, at: [f64; 2]) -> Value {
    json!({ "tick": tick, "burst": { "point": at, "weapon": "tank_he" } })
}

fn battle(props: Value, forests: Value, units: Value, events: Value) -> Battle {
    let map = json!({ "size": [1200, 600], "height_grid_m": 4, "slope_cutoff_deg": 35,
                      "props": props, "forests": forests });
    Battle::new(
        &common::scenario_with(&map.to_string(), units, events, json!([])),
        1,
    )
}

fn run(b: &mut Battle, ticks: u64) {
    for _ in 0..ticks {
        b.step();
    }
}

/// A red squad far off, so no side sees the props unless placed to.
fn far() -> Value {
    json!([{ "side": "red", "kind": "rifle", "position": [1150, 550], "engagement": "return_fire_only" }])
}

#[test]
fn a_burst_wears_each_destroyable_prop_by_its_distance_and_never_the_rest() {
    let (sd, radius) = he();
    // Sandbags 3 m from the burst, a crate 20 m off, a tooth beside it.
    let mut b = battle(
        json!([
            prop("sandbags", [303.5, 300.0], [0.5, 2.0, 0.5]),
            prop("crate", [320.0, 300.0], [0.5, 0.5, 0.5]),
            prop("tooth", [300.0, 303.0], [0.6, 0.6, 0.6]),
        ]),
        json!([]),
        far(),
        json!([burst(1, [300.0, 300.0])]),
    );
    run(&mut b, 3);
    let hp = row("sandbags")["hp"].as_f64().unwrap();
    let left = b.structures().hp(b.world(), 0).unwrap();
    assert!(
        (hp - left - sd * (1.0 - 3.0 / radius)).abs() < 1e-9,
        "{left}"
    );
    // Out of reach: whole. No integrity: never worn, still standing.
    assert_eq!(b.structures().hp(b.world(), 1), row("crate")["hp"].as_f64());
    assert_eq!(b.structures().hp(b.world(), 2), None);
    assert!(b.world().prop(2).is_some());
}

#[test]
fn a_direct_round_wears_the_struck_prop_by_its_armour() {
    // A tank fires at the ground just past a jeep wreck's side: the rounds
    // its dispersion puts into the wreck wear it by their structural damage
    // times its armour, and every burst off it by its distance, no more.
    let armor = row("jeep_wreck")["armor"].as_f64().unwrap();
    assert!(armor < 1.0, "the wreck is armoured");
    let mut b = battle(
        json!([prop("jeep_wreck", [300.0, 300.0], [2.0, 1.0, 1.0])]),
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [1150, 550], "engagement": "return_fire_only" },
        ]),
        json!([]),
    );
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [300.0, 301.6, 0.0],
            },
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let weapon = |name: &str| {
        let w = &rules()["weapons"][name];
        (
            w["structural_damage"].as_f64().unwrap_or(0.0),
            w["blast_radius_m"].as_f64().unwrap_or(0.0),
        )
    };
    let wreck = b.world().prop(0).unwrap().footprint();
    let mut owners: BTreeMap<_, String> = BTreeMap::new();
    let (mut direct, mut hp) = (0, b.structures().hp(b.world(), 0).unwrap());
    for _ in 0..1200 {
        b.step();
        let mut expected = 0.0;
        for e in b.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            let (sd, radius) = weapon(owners.get(&i.projectile).expect("a round seen in flight"));
            if i.struck == Struck::Prop(0) {
                direct += 1;
                expected += sd * armor;
            } else if i.detonated {
                let r = wreck.distance((i.point + i.normal * 0.05).xy());
                if r < radius {
                    expected += sd * (1.0 - r / radius);
                }
            }
        }
        for (p, r) in b.rounds() {
            owners
                .entry(p.id)
                .or_insert_with(|| b.arsenal().weapons[r.weapon].id.clone());
        }
        let Some(now) = b.structures().hp(b.world(), 0) else {
            break;
        };
        assert!(
            (hp - now - expected).abs() < 1e-9,
            "{hp} → {now}, expected {expected}"
        );
        hp = now;
    }
    assert!(direct > 0, "some round met the wreck");
}

#[test]
fn destroyed_props_become_their_rows_state() {
    // Two bursts on each: enough for every kind here.
    let at = |x: f64| [x, 300.0];
    let events: Vec<Value> = [300.0, 400.0, 500.0, 600.0, 700.0]
        .iter()
        .flat_map(|&x| {
            [
                burst(1, at(x)),
                burst(2, at(x)),
                burst(3, at(x)),
                burst(4, at(x)),
            ]
        })
        .collect();
    let mut b = battle(
        json!([
            prop("crate", at(300.0), [0.5, 0.5, 0.5]),
            prop("sandbags", at(400.0), [2.0, 0.4, 0.5]),
            prop("tank_wreck", at(500.0), [3.5, 1.8, 1.2]),
            prop("tooth", at(600.0), [0.6, 0.6, 0.6]),
        ]),
        json!([{ "rect": [680, 280, 40, 40], "density": "medium", "canopy_height_m": 12,
                 "trunk_radius_m": 0.35, "trunk_height_m": 6, "trunk_clearance_m": 1.5 }]),
        far(),
        json!(events),
    );
    let trunks = |b: &Battle| {
        b.world()
            .props()
            .filter(|p| p.kind == PropKind::Trunk && (p.center - v2(700.0, 300.0)).length() < 3.0)
            .map(|p| p.center)
            .collect::<Vec<_>>()
    };
    let felled = trunks(&b);
    assert!(!felled.is_empty(), "a trunk stands by the last burst");
    run(&mut b, 6);
    let w = b.world();
    // The crate is removed.
    assert!(w.prop(0).is_none());
    assert!(!w.props().any(|p| p.kind == PropKind::Crate));
    // The sandbags are rubble on their plan: light cover, blocking nothing.
    let rubble = w
        .props()
        .find(|p| p.kind == PropKind::Rubble)
        .expect("rubble");
    assert_eq!(b.structures().replaced_by(rubble.id), Some(1));
    assert_eq!(rubble.center, v2(400.0, 300.0));
    assert!(!rubble.blocks(MoverClass::Infantry) && !rubble.blocks(MoverClass::Vehicle));
    assert_eq!(
        rubble.body.cover_tier,
        Some(contract::scenario::CoverTier::Light)
    );
    // The tank wreck is a lighter wreck on its plan, lower.
    let lighter = w
        .props()
        .find(|p| b.structures().replaced_by(p.id) == Some(2))
        .expect("a lighter wreck");
    assert_eq!(lighter.kind, PropKind::SupplyWreck);
    assert!(lighter.half.z < 1.2);
    // The tooth stands: ordinary fire never destroys it (Q18).
    assert!(w.prop(3).is_some());
    // The trees are down and their spot is open ground.
    assert!(trunks(&b).is_empty());
    for p in felled {
        assert!(!w.forest_ground(p.x, p.y), "{p:?} is still forest ground");
        assert!(w.foliage_at(p.x, p.y).is_open());
        assert!(w.cleared(p.x, p.y));
    }
}

/// Sandbags destroyed at tick 1, and blue's squad `at` some distance.
fn sandbags_destroyed(blue_at: [f64; 2], destroy: bool) -> Battle {
    let events = if destroy {
        json!([burst(1, [400.0, 300.0]), burst(2, [400.0, 300.0])])
    } else {
        json!([])
    };
    let mut b = battle(
        json!([
            prop("sandbags", [400.0, 300.0], [2.0, 0.4, 0.5]),
            prop("crate", [400.0, 296.0], [0.5, 0.5, 0.5]),
        ]),
        json!([]),
        json!([
            { "side": "blue", "kind": "rifle", "position": blue_at, "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [1150, 550], "engagement": "return_fire_only" },
        ]),
        events,
    );
    run(&mut b, 60);
    b
}

#[test]
fn an_unseen_destruction_is_not_learned() {
    // Metamorphic: out of sight, blue's knowledge and plan are the same
    // whether or not the sandbags and crate were destroyed.
    let far_blue = [1150.0, 40.0];
    let (hit, calm) = (
        sandbags_destroyed(far_blue, true),
        sandbags_destroyed(far_blue, false),
    );
    assert!(hit.world().prop(0).is_none() && hit.world().prop(1).is_none());
    assert_eq!(
        hit.observe(Side::Blue).known_props,
        calm.observe(Side::Blue).known_props
    );
    assert_eq!(
        hit.navigation_revision(Side::Blue),
        calm.navigation_revision(Side::Blue)
    );
    // In sight, blue learns both: the rubble in place of the sandbags, and
    // that the crate is gone; its plan changes.
    let seen = sandbags_destroyed([380.0, 300.0], true);
    let known = &seen.observe(Side::Blue).known_props;
    let rubble = known
        .iter()
        .find(|p| p.kind == PropKind::Rubble)
        .expect("rubble");
    assert_eq!((rubble.replaces, rubble.destroyed), (Some(0), false));
    let gone = known
        .iter()
        .find(|p| p.replaces == Some(1))
        .expect("the crate");
    assert!(gone.destroyed);
    let calm_seen = sandbags_destroyed([380.0, 300.0], false);
    assert!(seen.navigation_revision(Side::Blue) > calm_seen.navigation_revision(Side::Blue));
}

#[test]
fn destruction_enters_the_digest_and_replays_exactly() {
    let run_one = |destroy| {
        let b = sandbags_destroyed([380.0, 300.0], destroy);
        (b.digest(), b)
    };
    let (d1, b) = run_one(true);
    let (d2, _) = run_one(true);
    let (calm, _) = run_one(false);
    assert_eq!(d1, d2);
    assert_ne!(d1, calm);
    // Worn but standing enters the digest too.
    let mut worn = battle(
        json!([prop("sandbags", [400.0, 300.0], [2.0, 0.4, 0.5])]),
        json!([]),
        far(),
        json!([burst(1, [405.0, 300.0])]),
    );
    let mut whole = battle(
        json!([prop("sandbags", [400.0, 300.0], [2.0, 0.4, 0.5])]),
        json!([]),
        far(),
        json!([burst(1, [900.0, 300.0])]),
    );
    run(&mut worn, 3);
    run(&mut whole, 3);
    assert!(worn.structures().hp(worn.world(), 0) < whole.structures().hp(whole.world(), 0));
    assert_ne!(worn.digest(), whole.digest());
    // And the destroyed battle replays to the same digest.
    let replay = b.replay();
    let setup = common::scenario_with(
        &json!({ "size": [1200, 600], "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": [prop("sandbags", [400.0, 300.0], [2.0, 0.4, 0.5]),
                           prop("crate", [400.0, 296.0], [0.5, 0.5, 0.5])],
                 "forests": [] })
        .to_string(),
        json!([
            { "side": "blue", "kind": "rifle", "position": [380.0, 300.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [1150, 550], "engagement": "return_fire_only" },
        ]),
        json!([burst(1, [400.0, 300.0]), burst(2, [400.0, 300.0])]),
        json!([]),
    );
    let mut again = Battle::from_replay(&setup, &replay).unwrap();
    run(&mut again, 60);
    assert_eq!(again.digest(), d1);
}
