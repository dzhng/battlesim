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
fn no_seat_events_preserve_the_pre_feature_tick_digests_and_replay_shape() {
    use sha2::{Digest, Sha256};
    // SHA-256 of the complete 601-tick digest streams captured at be8bb7e3e,
    // before any simulation edit. This pins outcomes, not tuning values.
    for (seed, expected) in [
        (
            1,
            "712993411397862dccb7983b547168d7f47f3f98cfb654dcae12086fc6a0afd6",
        ),
        (
            7,
            "53de4166a3439cff168027d03d0f7b03fad9be24886f93f1744b3cc69089c607",
        ),
        (
            19,
            "48963fc827ceff411248257c3cd125b35b19c8dfd3c300a876d005371c8c887f",
        ),
    ] {
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
        let mut digests = vec![format!("{:016x}", battle.digest())];
        for _ in 0..600 {
            battle.step();
            digests.push(format!("{:016x}", battle.digest()));
        }
        let encoded = serde_json::to_vec(&digests).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(encoded)),
            expected,
            "seed {seed}"
        );
        assert!(battle.replay().seat_events.is_empty());
        assert!(serde_json::to_value(battle.replay())
            .unwrap()
            .get("seat_events")
            .is_none());
    }
}
