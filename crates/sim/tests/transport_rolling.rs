//! Squads climb on and off a carrier rolling no faster than the rules'
//! climbing speed; a faster carrier takes no one on or off.
use contract::command::{CommandEnvelope, Order};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;
use sim::hold::Phase;

use crate::common;

/// The climbing speed every test here sets, m/s.
const CLIMB: f64 = 2.5;
/// A creeping carrier's top speed off-road, km/h: 2 m/s, under [`CLIMB`].
const CREEP_KMH: f64 = 7.2;

fn command(seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued: false,
    }
}

/// An open field with a blue carrier (unit 0) at (200, 200) facing east and
/// a blue squad (unit 1) at `squad`. `creep` caps the carrier at
/// [`CREEP_KMH`]; otherwise it drives at its full off-road speed, well over
/// [`CLIMB`].
fn battle(squad: [f64; 2], creep: bool) -> Battle {
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "props":[],"buildings":[]});
    let mut setup = common::scenario_with(
        &map.to_string(),
        json!([
            {"side":"blue","kind":"test_apc","position":[200,200],"yaw":0},
            {"side":"blue","kind":"test_rifle","position":squad},
            {"side":"red","kind":"test_rifle","position":[750,550]}
        ]),
        json!([]),
        json!([]),
    );
    let mut rules = serde_json::to_value(&setup.rules).unwrap();
    rules["carriage"]["climb_speed_mps"] = json!(CLIMB);
    if creep {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_apc",
            json!({"mobility":{"wheeled":{"offroad_kmh": CREEP_KMH}}}),
        );
    }
    setup.rules = serde_json::from_value(rules).unwrap();
    Battle::new(&setup, 3)
}

fn drive(b: &mut Battle, seq: u64, goal: [f64; 2]) {
    let ack = b.accept(command(
        seq,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: seq,
            goal,
            route: Default::default(),
            direction: Default::default(),
            facing: None,
        },
    ));
    assert_eq!(ack.error, None, "{ack:?}");
}

fn board(b: &mut Battle, seq: u64) {
    let ack = b.accept(command(
        seq,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: seq,
        },
    ));
    assert_eq!(ack.error, None, "{ack:?}");
}

fn unload_at(b: &mut Battle, seq: u64, goal: [f64; 2]) {
    let ack = b.accept(command(
        seq,
        Order::Unload {
            into: None,
            units: vec![UnitId(0)],
            goal: Some(goal),
            gesture: seq,
            route: Default::default(),
        },
    ));
    assert_eq!(ack.error, None, "{ack:?}");
}

fn phase(b: &Battle) -> Option<Phase> {
    b.unit(UnitId(1)).unwrap().hold.as_ref().map(|h| h.phase)
}

fn speed(b: &Battle) -> f64 {
    b.unit(UnitId(0)).unwrap().drive_speed_mps.abs()
}

/// A loaded carrier: the squad beside it boards while it stands.
fn loaded(creep: bool) -> Battle {
    let mut b = battle([195.0, 205.0], creep);
    board(&mut b, 1);
    let aboard = (0..b.rules().tick_hz * 30).any(|_| {
        b.step();
        phase(&b) == Some(Phase::Inside)
    });
    assert!(aboard, "the squad boards the parked carrier");
    b
}

#[test]
fn a_squad_climbs_into_a_carrier_creeping_past_it() {
    let mut b = battle([215.0, 204.0], true);
    drive(&mut b, 1, [400.0, 200.0]);
    board(&mut b, 2);
    let boarded = (0..b.rules().tick_hz * 30).find_map(|_| {
        b.step();
        (phase(&b) == Some(Phase::Inside)).then(|| speed(&b))
    });
    let speed = boarded.expect("the squad climbs in");
    assert!(
        speed > 1.0,
        "the carrier was still rolling as the squad got in: {speed:.2} m/s"
    );
}

#[test]
fn a_carrier_faster_than_climbing_speed_takes_no_one_aboard_until_it_slows() {
    let mut b = battle([215.0, 204.0], false);
    drive(&mut b, 1, [400.0, 200.0]);
    board(&mut b, 2);
    let mut aboard = false;
    for _ in 0..b.rules().tick_hz * 150 {
        b.step();
        if phase(&b).is_some() {
            assert!(
                speed(&b) <= CLIMB + 1e-9,
                "climbing in at {:.2} m/s",
                speed(&b)
            );
        }
        if phase(&b) == Some(Phase::Inside) {
            aboard = true;
            break;
        }
    }
    assert!(aboard, "the squad gets in once the carrier stops");
}

#[test]
fn a_creeping_carrier_lets_its_riders_off_without_stopping() {
    let mut b = loaded(true);
    unload_at(&mut b, 2, [240.0, 200.0]);
    let off = (0..b.rules().tick_hz * 60).find_map(|_| {
        b.step();
        phase(&b).is_none().then(|| speed(&b))
    });
    let speed = off.expect("the riders get off");
    assert!(
        speed > 1.0,
        "the carrier was still rolling as its riders got off: {speed:.2} m/s"
    );
    let carrier = b.unit(UnitId(0)).unwrap().position.xy();
    let squad = b.unit(UnitId(1)).unwrap().position.xy();
    assert!(
        (carrier - squad).length() < 10.0,
        "the riders stand beside the carrier where they got off"
    );
}

#[test]
fn a_carrier_faster_than_climbing_speed_drops_no_one() {
    let mut b = loaded(false);
    unload_at(&mut b, 2, [300.0, 200.0]);
    let mut off = false;
    for _ in 0..b.rules().tick_hz * 60 {
        b.step();
        if matches!(phase(&b), Some(Phase::Exiting(_)) | None) {
            assert!(
                speed(&b) <= CLIMB + 1e-9,
                "getting off at {:.2} m/s",
                speed(&b)
            );
        }
        if phase(&b).is_none() {
            off = true;
            break;
        }
    }
    assert!(off, "the riders get off once the carrier slows");
}
