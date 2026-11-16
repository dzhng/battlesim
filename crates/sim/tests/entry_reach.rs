//! A squad close enough to a carrier or a building gets in, whatever the
//! host is doing and whether or not a route round it can be planned.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::MoveState;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

fn command(seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued: false,
    }
}

fn map(props: Value, buildings: Value) -> String {
    json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "props": props, "buildings": buildings})
    .to_string()
}

fn inside(b: &Battle, id: u32) -> bool {
    b.unit(UnitId(id))
        .and_then(|u| u.hold.as_ref())
        .is_some_and(|h| h.phase == sim::hold::Phase::Inside)
}

fn within(b: &mut Battle, seconds: u32, id: u32) -> bool {
    (0..b.rules().tick_hz * seconds).any(|_| {
        b.step();
        inside(b, id)
    })
}

/// A carrier whose own drive is blocked still takes aboard a squad that
/// comes up beside it. The carrier drives up a lane jammed by two crates
/// side by side, which it can shove one of but not one into the other,
/// gives up there, and keeps its blocked order.
#[test]
fn a_squad_boards_a_carrier_whose_route_is_blocked() {
    let props = json!([
        {"kind":"wall", "center":[600,55], "yaw":0, "half_extents":[100,55,2]},
        {"kind":"wall", "center":[600,185], "yaw":0, "half_extents":[100,55,2]},
        {"kind":"crate", "center":[600,120], "yaw":0, "half_extents":[1.5,9.9,2]},
        {"kind":"crate", "center":[603,120], "yaw":0, "half_extents":[1.5,9.9,2]}
    ]);
    let mut b = Battle::new(
        &common::scenario_with(
            &json!({"size":[1200,240], "fog_cell_m":8, "height_grid_m":4,
                "slope_cutoff_deg":35, "props":props})
            .to_string(),
            json!([
                {"side":"blue","kind":"test_apc","position":[540,120],"yaw":0},
                {"side":"blue","kind":"test_rifle","position":[520,120]},
                {"side":"red","kind":"test_rifle","position":[1150,200]}
            ]),
            json!([]),
            json!([]),
        ),
        7,
    );
    let drive = b.accept(command(
        1,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [1100.0, 120.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    ));
    assert_eq!(drive.error, None, "{drive:?}");
    let blocked = (0..b.rules().tick_hz * 60).any(|_| {
        b.step();
        b.observe(Side::Blue)
            .own
            .iter()
            .any(|u| u.id == UnitId(0) && u.state == MoveState::RouteBlocked)
    });
    assert!(blocked, "the carrier gives up on the jammed lane");
    let board = b.accept(command(
        2,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 2,
        },
    ));
    assert_eq!(board.error, None, "{board:?}");
    assert!(
        within(&mut b, 60, 1),
        "the squad never boarded the blocked carrier"
    );
}

/// A squad already standing within entry reach of a building's wall enters
/// it from where it stands.
#[test]
fn a_squad_at_the_wall_enters_without_a_planned_approach() {
    let setup = common::scenario_with(
        &map(
            json!([]),
            json!([common::building(
                0,
                "china-apartment-point-20x20-7f",
                [400.0, 300.0],
                0.0
            )]),
        ),
        json!([{"side":"blue","kind":"test_recon","position":[387.0, 300.0]}]),
        json!([]),
        json!([]),
    );
    let mut b = Battle::new(&setup, 7);
    let ack = b.accept(command(
        1,
        Order::OccupyBuilding {
            units: vec![UnitId(0)],
            building: 0,
            gesture: 1,
        },
    ));
    assert_eq!(ack.error, None, "{ack:?}");
    let entries = ack.building.as_ref().map(|p| p.entries.clone());
    assert_eq!(
        entries.map(|e| e.iter().map(|e| e.unit).collect::<Vec<_>>()),
        Some(vec![UnitId(0)]),
        "{ack:?}"
    );
    assert!(within(&mut b, 30, 0), "the squad at the wall never went in");
}
