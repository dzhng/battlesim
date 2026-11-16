//! Returning a carrier to base (a refund): its passengers go with it, and
//! one queued behind other orders waits for them.
use contract::command::{CommandEnvelope, Order, OrderError, RoutePolicy};
use contract::ids::{Side, UnitId};
use sim::battle::Battle;

use crate::common::Seat;
use crate::transport::{side_command, transport_skirmish_setup};

#[test]
fn refunding_a_loaded_carrier_refunds_its_passengers() {
    let mut battle = Battle::new(&transport_skirmish_setup(), 41);
    assert!(battle
        .accept(side_command(
            Side::Blue,
            1,
            Order::purchase("test_apc", [300.0, 100.0]),
        ))
        .error
        .is_none());
    assert!(battle
        .accept(side_command(
            Side::Blue,
            2,
            Order::purchase("test_recon", [300.0, 100.0]),
        ))
        .error
        .is_none());
    battle.accept(side_command(Side::Blue, 3, Order::Ready));
    battle.accept(side_command(Side::Red, 1, Order::Ready));
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(1)).is_some() {
            break;
        }
    }
    assert!(battle.unit(UnitId(0)).is_some() && battle.unit(UnitId(1)).is_some());
    assert!(battle
        .accept(side_command(
            Side::Blue,
            4,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1,
            },
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());
    let refused = battle.accept(side_command(
        Side::Blue,
        5,
        Order::Refund {
            units: vec![UnitId(1)],
        },
    ));
    assert_eq!(refused.error, Some(OrderError::Aboard { unit: UnitId(1) }));
    let before = battle.observe(Side::Blue).seat().credits;
    assert!(battle
        .accept(side_command(
            Side::Blue,
            6,
            Order::Refund {
                units: vec![UnitId(0)]
            }
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().retired {
            break;
        }
    }
    let after = battle.observe(Side::Blue).seat().credits;
    assert!(after > before);
    assert!(battle.unit(UnitId(0)).unwrap().retired);
    assert!(battle.unit(UnitId(1)).unwrap().retired);
}

#[test]
fn a_return_to_base_queued_behind_an_unload_leaves_once_the_squad_is_off() {
    let mut battle = Battle::new(&transport_skirmish_setup(), 41);
    for (seq, kind) in [(1, "test_apc"), (2, "test_recon")] {
        let buy = Order::purchase(kind, [300.0, 100.0]);
        assert!(battle
            .accept(side_command(Side::Blue, seq, buy))
            .error
            .is_none());
    }
    battle.accept(side_command(Side::Blue, 3, Order::Ready));
    battle.accept(side_command(Side::Red, 1, Order::Ready));
    let arrived = (0..battle.rules().tick_hz * 30).any(|_| {
        battle.step();
        battle.unit(UnitId(1)).is_some()
    });
    assert!(arrived);
    let board = Order::Board {
        units: vec![UnitId(1)],
        carrier: UnitId(0),
        gesture: 1,
    };
    assert!(battle
        .accept(side_command(Side::Blue, 4, board))
        .error
        .is_none());
    let aboard = (0..battle.rules().tick_hz * 60).any(|_| {
        battle.step();
        !battle.unit(UnitId(1)).unwrap().on_field()
    });
    assert!(aboard);
    let drop_at = [300.0, 300.0];
    let unload = Order::Unload {
        into: None,
        units: vec![UnitId(0)],
        goal: Some(drop_at),
        gesture: 2,
        route: RoutePolicy::Shortest,
    };
    assert!(battle
        .accept(side_command(Side::Blue, 5, unload))
        .error
        .is_none());
    let home = CommandEnvelope {
        queued: true,
        ..side_command(
            Side::Blue,
            6,
            Order::Refund {
                units: vec![UnitId(0)],
            },
        )
    };
    assert!(battle.accept(home).error.is_none());
    let retired = (0..battle.rules().tick_hz * 120).any(|_| {
        battle.step();
        battle.unit(UnitId(0)).unwrap().retired
    });
    assert!(retired, "the carrier never went home");
    let squad = battle.unit(UnitId(1)).unwrap();
    assert!(
        squad.on_field() && !squad.retired,
        "the squad went home aboard"
    );
    let from_drop = (squad.position.xy() - sim::math::v2(drop_at[0], drop_at[1])).length();
    assert!(
        from_drop < 15.0,
        "the squad got off {from_drop} m from the drop"
    );
}
