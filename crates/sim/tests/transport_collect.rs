//! A carrier collects squads: it drives over, and each squad it lists gets
//! in, walking over when out of reach; one another carrier took first, or
//! listed twice, is refused once.
use contract::command::{CommandEnvelope, Order, OrderError};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;

fn setup(units: serde_json::Value) -> contract::scenario::ScenarioDefinition {
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]});
    crate::common::scenario_with(&map.to_string(), units, json!([]), json!([]))
}

fn command(seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued: false,
    }
}

#[test]
fn a_collected_squad_out_of_reach_walks_over_before_it_boards() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[250,200]},
            {"side":"blue","kind":"test_recon","position":[220,200]},
            {"side":"blue","kind":"test_recon","position":[220,240]}
        ])),
        10,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Collect {
                units: vec![UnitId(0)],
                squads: vec![UnitId(1), UnitId(2)],
                gesture: 1
            }
        ))
        .error
        .is_none());
    // A soldier on foot covers well under a metre a tick; the far squad is
    // 40 m from the first one the carrier reaches.
    let mut last = battle.unit(UnitId(2)).unwrap().position.xy();
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        let squad = battle.unit(UnitId(2)).unwrap();
        if !squad.on_field() {
            break;
        }
        let moved = (squad.position.xy() - last).length();
        assert!(moved < 1.0, "the far squad jumped {moved} m in one tick");
        last = squad.position.xy();
    }
    for _ in 0..(battle.rules().tick_hz * 10) {
        battle.step();
    }
    for id in [UnitId(1), UnitId(2)] {
        assert_eq!(
            battle.unit(id).unwrap().hold.as_ref().map(|h| h.phase),
            Some(sim::hold::Phase::Inside),
            "every collected squad ends up aboard"
        );
    }
}

/// Carrier 0 sent from 200 m to collect `squads`; squad 1 climbs into carrier
/// 2 beside it first. Returns the battle 5 s later and every refusal blue
/// heard about squad 1.
fn collected_squad_boards_another_carrier(
    squads: Vec<UnitId>,
) -> (Battle, Vec<contract::command::Refusal>) {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[400,200]},
            {"side":"blue","kind":"test_recon","position":[195,200]},
            {"side":"blue","kind":"test_apc","position":[200,200]}
        ])),
        7,
    );
    let collect = battle.accept(command(
        1,
        Order::Collect {
            units: vec![UnitId(0)],
            squads,
            gesture: 1,
        },
    ));
    assert_eq!(collect.error, None);
    let board = battle.accept(command(
        2,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(2),
            gesture: 2,
        },
    ));
    assert_eq!(board.error, None);
    let mut refusals = Vec::new();
    for _ in 0..(battle.rules().tick_hz * 5) {
        battle.step();
        refusals.extend(
            (battle.observe(Side::Blue).refusals.iter())
                .filter(|r| r.unit == UnitId(1))
                .cloned(),
        );
    }
    (battle, refusals)
}

#[test]
fn a_carrier_stops_driving_to_a_squad_another_carrier_took() {
    let (battle, refusals) = collected_squad_boards_another_carrier(vec![UnitId(1)]);
    assert!(
        refusals
            .iter()
            .any(|r| r.reason == OrderError::Aboard { unit: UnitId(1) }),
        "the carrier was never told the squad is aboard another"
    );
    let carrier = battle.unit(UnitId(0)).unwrap();
    assert!(
        carrier.orders.is_empty(),
        "the carrier still drives to where the squad stood: {:?}",
        carrier.orders
    );
}

#[test]
fn a_squad_listed_twice_for_collection_is_refused_once() {
    let (_, refusals) = collected_squad_boards_another_carrier(vec![UnitId(1), UnitId(1)]);
    assert_eq!(
        refusals,
        vec![contract::command::Refusal {
            unit: UnitId(1),
            reason: OrderError::Aboard { unit: UnitId(1) },
        }]
    );
}
