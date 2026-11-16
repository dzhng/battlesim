//! Administrative events reproduce at exactly the same tick as live play.
use crate::seat_hand_over::{buy, setup};
use contract::command::Order;
use contract::ids::Side;
use contract::players::ControllerKind;
use sim::battle::{Battle, SeatEvent};

#[test]
fn hand_over_replays_every_tick_and_tampering_changes_the_battle() {
    let setup = setup();
    let mut live = Battle::new(&setup, 8);
    buy(&mut live, "blue-1", "test_tank");
    let mut digests = Vec::new();
    for tick in 1..=120 {
        if tick == 41 {
            live.hand_over("blue-1").unwrap();
        }
        live.step();
        digests.push(live.digest());
    }
    let record = live.replay();
    assert_eq!(
        record.seat_events,
        vec![SeatEvent::HandOver {
            tick: 41,
            player: "blue-1".into()
        }]
    );
    let mut replay = Battle::from_replay(&setup, &record).unwrap();
    for digest in &digests {
        replay.step();
        assert_eq!(replay.digest(), *digest, "tick {}", replay.tick());
    }
    for event in [
        SeatEvent::HandOver {
            tick: 42,
            player: "blue-1".into(),
        },
        SeatEvent::HandOver {
            tick: 41,
            player: "blue-2".into(),
        },
    ] {
        let mut tampered = record.clone();
        tampered.seat_events[0] = event;
        let mut replay = Battle::from_replay(&setup, &tampered).unwrap();
        let mut different = false;
        for digest in &digests {
            replay.step();
            different |= replay.digest() != *digest;
        }
        assert!(different, "a changed event reproduced every tick");
    }
}

#[test]
fn same_tick_events_apply_in_recording_order_after_order_admission() {
    let mut setup = setup();
    let mut third = setup.players[2].clone();
    third.id = "blue-3".into();
    setup.players.push(third);
    setup.skirmish.as_mut().unwrap().sites = sim::skirmish::synthetic_sites(
        setup.map.size,
        contract::players::entries_per_side(&setup.players),
    );
    let mut live = Battle::new(&setup, 9);
    // This purchase is admitted before both transfers and arrives afterwards.
    buy(&mut live, "blue-1", "test_tank");
    live.hand_over("blue-1").unwrap();
    live.hand_over("blue-2").unwrap();
    live.step();
    assert_eq!(
        live.unit(contract::ids::UnitId(0))
            .unwrap()
            .owner
            .as_deref(),
        Some("blue-3")
    );
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    replay.step();
    assert_eq!(replay.digest(), live.digest());
}

#[test]
fn a_battle_without_seat_events_records_none_and_its_replay_makes_it_again() {
    // While this was built, three battles' whole digest streams were pinned
    // here and matched staging's own at every merge, the last at 2026-11-16:
    // adding seat events changed no battle that has none. The pins moved
    // with every change to movement or the AI, so they are retired; the
    // simulation's own digests and parity records pin outcomes.
    for seed in [1, 7, 19] {
        let mut setup = crate::skirmish::setup();
        if seed == 19 {
            setup.players[0].controller = ControllerKind::ActiveAi;
            setup.players[1].controller = ControllerKind::ActiveAi;
            setup.players[1].faction = setup.players[0].faction;
        }
        let mut battle = Battle::new(&setup, seed);
        if seed != 19 {
            for order in [
                Order::ConfirmPurchase {
                    variant: "test_tank".into(),
                    carrier: None,
                    destination: [400.0, 280.0],
                },
                Order::Ready,
            ] {
                assert_eq!(
                    battle
                        .accept_order(Side::Blue, Some("blue-1".into()), order, false)
                        .error,
                    None
                );
            }
            assert_eq!(
                battle
                    .accept_order(Side::Red, Some("red-1".into()), Order::Ready, false)
                    .error,
                None
            );
        }
        // From the first tick on: before it, the live battle already holds
        // the orders a replay has yet to give itself.
        let mut digests = Vec::new();
        for _ in 0..600 {
            battle.step();
            digests.push(battle.digest());
        }
        assert!(battle.replay().seat_events.is_empty());
        assert!(serde_json::to_value(battle.replay())
            .unwrap()
            .get("seat_events")
            .is_none());
        let mut again = Battle::from_replay(&setup, &battle.replay()).unwrap();
        let mut replayed = Vec::new();
        for _ in 0..600 {
            again.step();
            replayed.push(again.digest());
        }
        assert_eq!(replayed, digests, "seed {seed}");
    }
}
