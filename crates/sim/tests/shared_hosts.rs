//! Shared hosts: several squads share a building or a carrier while seats
//! remain, driven through commands, stepping and the side's observation.
use contract::command::{CommandEnvelope, Order, OrderError};
use contract::ids::{Side, UnitId};
use contract::observation::{HoldPhase, HostRef};
use serde_json::json;
use sim::battle::Battle;
use sim::garrison::seats_for_state;

use crate::common;

const TEMPLATE: &str = "china-apartment-point-20x20-7f";
const BUILDING: u32 = 0;

/// Two blue rifle squads: `A` beside the building, `B` well to the west.
const A: u32 = 0;
const B: u32 = 1;

/// A battle whose building seats one of the two squads but not both: the
/// test sets the cap itself from the squads' own size.
fn relief_battle() -> Battle {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, TEMPLATE, [400.0, 300.0], 0.0)] })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": "test_rifle", "position": [370.0, 300.0] },
        { "side": "blue", "kind": "test_rifle", "position": [220.0, 300.0] },
    ]);
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    let squad = Battle::new(&setup, 1)
        .unit(UnitId(A))
        .unwrap()
        .members
        .len() as u32;
    setup.rules.buildings.capacity_soldiers = 2 * squad - 1;
    Battle::new(&setup, 1)
}

fn occupy(unit: u32) -> Order {
    serde_json::from_value(json!({
        "kind": "occupy_building", "units": [unit], "building": BUILDING, "gesture": unit + 1
    }))
    .unwrap()
}

/// Send a blue order; it must be accepted with no unit refused.
fn accepted(b: &mut Battle, seq: u64, order: Order) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued: false,
    });
    assert!(ack.error.is_none() && ack.refused.is_empty(), "{ack:?}");
}

fn living(b: &Battle, id: u32) -> usize {
    b.unit(UnitId(id))
        .map_or(0, |u| u.members.iter().filter(|s| s.alive()).count())
}

/// The unit's phase in the building, as its side sees it.
fn phase(b: &Battle, id: u32) -> Option<HoldPhase> {
    let unit = b
        .observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))?;
    let hold = unit.hold?;
    matches!(hold.host, HostRef::Building { id, .. } if id == BUILDING).then_some(hold.phase)
}

#[test]
#[ignore = "shared hosts: green from slice 03"]
fn a_squad_ordered_into_a_held_house_walks_in_when_its_holder_leaves() {
    let mut b = relief_battle();
    let building = b.world().prop(BUILDING).cloned().unwrap();
    let seats = seats_for_state(b.world(), &building, b.rules()).len();
    let (a, squad_b) = (living(&b, A), living(&b, B));
    assert!(
        a <= seats && squad_b <= seats && a + squad_b > seats,
        "the building seats either squad but not both: {a} + {squad_b} into {seats}"
    );
    accepted(&mut b, 1, occupy(A));
    let limit = 3000;
    let mut t = 0;
    while phase(&b, A) != Some(HoldPhase::Inside) {
        assert!(t < limit, "A inside within {limit} ticks");
        b.step();
        t += 1;
    }

    // B is ordered into the full house, then A is told to leave: the player's
    // relief manoeuvre. Both commands are accepted.
    accepted(&mut b, 2, occupy(B));
    accepted(
        &mut b,
        3,
        Order::ExitBuilding {
            units: vec![UnitId(A)],
        },
    );

    let mut refusals: Vec<OrderError> = Vec::new();
    let (mut a_left, mut b_stepped_in) = (false, false);
    for t in 0..limit {
        b.step();
        let frame = b.observe(Side::Blue);
        refusals.extend(
            frame
                .refusals
                .iter()
                .filter(|r| r.unit == UnitId(B))
                .map(|r| r.reason.clone()),
        );
        let (pa, pb) = (phase(&b, A), phase(&b, B));
        a_left |= pa == Some(HoldPhase::Exiting);
        if pb.is_some() {
            b_stepped_in = true;
            assert!(
                a_left,
                "tick {t}: B stepped in ({pb:?}) before A began stepping out"
            );
        }
        // A seat is taken from the moment a squad starts stepping in; a squad
        // stepping out still stands at its windows.
        let taken = |count: &[HoldPhase]| -> usize {
            [(A, pa), (B, pb)]
                .iter()
                .filter(|(_, p)| p.is_some_and(|p| count.contains(&p)))
                .map(|&(id, _)| living(&b, id))
                .sum()
        };
        assert!(
            taken(&[HoldPhase::Entering, HoldPhase::Inside]) <= seats,
            "tick {t}: seats claimed past the {seats} the building has"
        );
        assert!(
            taken(&[HoldPhase::Inside, HoldPhase::Exiting]) <= seats,
            "tick {t}: more soldiers at windows than the {seats} the building has"
        );
        if pa.is_none() && pb == Some(HoldPhase::Inside) {
            break;
        }
    }
    assert!(b_stepped_in, "B never started stepping in");
    assert_eq!(refusals, [], "no refusal is published for B");
    assert_eq!(phase(&b, A), None, "A ends outside");
    assert_eq!(phase(&b, B), Some(HoldPhase::Inside), "B ends inside");
    assert_eq!(living(&b, B), squad_b, "B is inside whole");
}

/// One building order names three squads standing round the house: each
/// gets its own approach and walks to the house to step in.
#[test]
fn every_squad_a_building_order_names_walks_to_the_house() {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, TEMPLATE, [400.0, 300.0], 0.0)] })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": "test_rifle", "position": [300.0, 300.0] },
        { "side": "blue", "kind": "test_recon", "position": [400.0, 180.0] },
        { "side": "blue", "kind": "test_recon", "position": [520.0, 320.0] },
    ]);
    let mut b = Battle::new(&common::scenario_with(&map, units, json!([]), json!([])), 1);
    let squads = [0, 1, 2];
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: serde_json::from_value(json!({
            "kind": "occupy_building", "units": squads, "building": BUILDING, "gesture": 1
        }))
        .unwrap(),
        queued: false,
    });
    assert!(ack.error.is_none() && ack.refused.is_empty(), "{ack:?}");
    let entries = ack.building.expect("a building plan").entries;
    assert_eq!(
        entries.iter().map(|e| e.unit.0).collect::<Vec<_>>(),
        squads,
        "one entry per squad"
    );
    // Each approach lies on the side of the house its squad comes from.
    for (e, from) in entries
        .iter()
        .zip([[300.0, 300.0], [400.0, 180.0], [520.0, 320.0]])
    {
        let (to_squad, to_approach) = (
            [from[0] - 400.0, from[1] - 300.0],
            [e.approach[0] - 400.0, e.approach[1] - 300.0],
        );
        assert!(
            to_squad[0] * to_approach[0] + to_squad[1] * to_approach[1] > 0.0,
            "{e:?} faces away from its squad at {from:?}"
        );
    }
    let mut stepped_in = [false; 3];
    for _ in 0..3000 {
        b.step();
        for (k, &id) in squads.iter().enumerate() {
            stepped_in[k] |= phase(&b, id).is_some();
        }
        if stepped_in.iter().all(|&s| s) {
            break;
        }
    }
    assert_eq!(stepped_in, [true; 3], "every squad reached the house");
}
