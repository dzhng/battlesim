//! Unloading into a host: a carrier sets its squads down beside a building
//! they garrison, or beside another carrier they board.
use contract::command::{CommandEnvelope, Order, RoutePolicy, UnloadInto};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;
use sim::hold::{Host, Phase};

use crate::buildings::compound_setup;

const CARRIER: UnitId = UnitId(0);
const SQUAD: UnitId = UnitId(1);

fn blue(seq: u64, order: Order, queued: bool) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued,
    }
}

/// Step until `done` holds, at most `seconds` of battle; whether it did.
fn until(battle: &mut Battle, seconds: u32, mut done: impl FnMut(&Battle) -> bool) -> bool {
    (0..battle.rules().tick_hz * seconds).any(|_| {
        battle.step();
        done(battle)
    })
}

fn hold_of(battle: &Battle, unit: UnitId) -> Option<(Host, Phase)> {
    let hold = battle.unit(unit)?.hold.as_ref()?;
    Some((hold.host, hold.phase))
}

/// The compound building's battle with `units`, the squad (unit 1) already
/// inside the carrier (unit 0).
fn loaded(units: serde_json::Value) -> Battle {
    let mut setup = compound_setup(json!([]));
    setup.units = serde_json::from_value(units).unwrap();
    let mut battle = Battle::new(&setup, 7);
    let board = Order::Board {
        units: vec![SQUAD],
        carrier: CARRIER,
        gesture: 1,
    };
    assert_eq!(battle.accept(blue(1, board, false)).error, None);
    assert!(
        until(&mut battle, 20, |b| hold_of(b, SQUAD)
            == Some((Host::Carrier(CARRIER), Phase::Inside))),
        "the squad never boarded"
    );
    battle
}

fn unload_into(into: UnloadInto, goal: [f64; 2]) -> Order {
    Order::Unload {
        units: vec![CARRIER],
        goal: Some(goal),
        gesture: 2,
        route: RoutePolicy::Shortest,
        into: Some(into),
    }
}

#[test]
fn a_helicopter_unloading_into_a_building_sets_its_squad_down_to_garrison_it() {
    let mut battle = loaded(json!([
        {"side":"blue","kind":"test_transport_heli","position":[300,300]},
        {"side":"blue","kind":"test_rifle","position":[300,300]},
    ]));
    // Clicked on the roof: the carrier sets down beside the building instead.
    let order = unload_into(UnloadInto::Building { building: 0 }, [400.0, 300.0]);
    let ack = battle.accept(blue(2, order, false));
    assert_eq!((ack.error, ack.refused), (None, vec![]));
    assert!(
        until(&mut battle, 60, |b| hold_of(b, SQUAD)
            == Some((Host::Building(0), Phase::Inside))),
        "the squad never garrisoned the building: {:?}",
        hold_of(&battle, SQUAD)
    );
}

#[test]
fn a_ground_carrier_unloading_into_a_building_parks_beside_it_for_the_squad_to_garrison() {
    let mut battle = loaded(json!([
        {"side":"blue","kind":"test_apc","position":[300,300]},
        {"side":"blue","kind":"test_recon","position":[300,300]},
    ]));
    let order = unload_into(UnloadInto::Building { building: 1 }, [404.0, 300.0]);
    let ack = battle.accept(blue(2, order, false));
    assert_eq!((ack.error, ack.refused), (None, vec![]));
    assert!(
        until(&mut battle, 60, |b| hold_of(b, SQUAD)
            == Some((Host::Building(0), Phase::Inside))),
        "the squad never garrisoned the building: {:?}",
        hold_of(&battle, SQUAD)
    );
}

#[test]
fn a_helicopter_unloading_into_another_carrier_sets_its_squad_down_to_board_it() {
    const APC: UnitId = UnitId(2);
    let mut battle = loaded(json!([
        {"side":"blue","kind":"test_transport_heli","position":[200,150]},
        {"side":"blue","kind":"test_rifle","position":[200,150]},
        {"side":"blue","kind":"test_apc","position":[250,450]},
    ]));
    let order = unload_into(UnloadInto::Carrier { carrier: APC }, [250.0, 450.0]);
    let ack = battle.accept(blue(2, order, false));
    assert_eq!((ack.error, ack.refused), (None, vec![]));
    assert!(
        until(&mut battle, 60, |b| hold_of(b, SQUAD)
            == Some((Host::Carrier(APC), Phase::Inside))),
        "the squad never boarded the other carrier: {:?}",
        hold_of(&battle, SQUAD)
    );
}

#[test]
fn a_carrier_cannot_unload_into_itself() {
    let mut battle = loaded(json!([
        {"side":"blue","kind":"test_apc","position":[300,300]},
        {"side":"blue","kind":"test_recon","position":[300,300]},
    ]));
    let order = unload_into(UnloadInto::Carrier { carrier: CARRIER }, [300.0, 300.0]);
    let ack = battle.accept(blue(2, order, false));
    assert_eq!(ack.error, Some(contract::command::OrderError::NotACarrier));
}

#[test]
fn squads_unload_only_into_their_own_players_carrier() {
    let mut setup = compound_setup(json!([]));
    setup.units = serde_json::from_value(json!([
        {"side":"blue","kind":"test_apc","position":[300,300],"owner":"blue-1"},
        {"side":"blue","kind":"test_recon","position":[300,300],"owner":"blue-1"},
        {"side":"blue","kind":"test_apc","position":[300,350],"owner":"blue-2"},
        {"side":"blue","kind":"test_apc","position":[300,250],"owner":"blue-1"},
    ]))
    .unwrap();
    let mut battle = Battle::new(&setup, 7);
    let as_blue_1 = |seq, order| CommandEnvelope {
        player: Some("blue-1".into()),
        ..blue(seq, order, false)
    };
    let board = Order::Board {
        units: vec![SQUAD],
        carrier: CARRIER,
        gesture: 1,
    };
    assert_eq!(battle.accept(as_blue_1(1, board)).error, None);
    assert!(until(&mut battle, 20, |b| hold_of(b, SQUAD)
        == Some((Host::Carrier(CARRIER), Phase::Inside))));
    let into = |carrier| unload_into(UnloadInto::Carrier { carrier }, [300.0, 300.0]);
    assert_eq!(
        battle.accept(as_blue_1(2, into(UnitId(2)))).error,
        Some(contract::command::OrderError::WrongPlayer { unit: UnitId(2) }),
        "a squad was sent into its ally's carrier"
    );
    assert_eq!(battle.accept(as_blue_1(3, into(UnitId(3)))).error, None);
}
