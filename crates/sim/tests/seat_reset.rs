//! A reset is an administrative fresh start, not a new player identity.
use crate::seat_hand_over::{buy, entered, order, seat, setup};
use contract::command::Order;
use contract::ids::Side;
use contract::players::ControllerKind;
use sim::battle::{Battle, SeatError, SeatEvent};

#[test]
fn reset_sets_exactly_the_requested_opening_wallet_then_earns_income() {
    for opening in [true, false] {
        let setup = setup();
        let rules = &setup.skirmish.as_ref().unwrap().rules;
        let expected = if opening {
            f64::from(rules.credits_per_minute) * f64::from(rules.starting_minutes)
        } else {
            0.0
        };
        assert!(f64::from(rules.credits_per_minute) * f64::from(rules.starting_minutes) > 0.0);
        let mut battle = Battle::new(&setup, 10);
        entered(&mut battle, "blue-1");
        battle.hand_over("blue-1").unwrap();
        battle.step();
        battle.reset_seat("blue-1", opening).unwrap();
        battle.step();
        assert_eq!(seat(&battle, "blue-1").unit_count, 0);
        assert!(seat(&battle, "blue-1").pending.is_empty());
        assert!(battle
            .observe(Side::Blue)
            .own
            .iter()
            .all(|u| u.owner != Some(contract::players::PlayerIndex(0))));
        assert_eq!(seat(&battle, "blue-1").credits, expected);
        let receiver = seat(&battle, "blue-2").credits;
        for _ in 0..battle.rules().tick_hz {
            battle.step();
        }
        let income = seat(&battle, "blue-1").credits - expected;
        assert!(income > 0.0);
        assert!((income - (seat(&battle, "blue-2").credits - receiver)).abs() < 0.000001);
    }
}

#[test]
fn resetting_units_or_pending_purchases_is_refused_without_mutation() {
    let mut setup = setup();
    setup.skirmish.as_mut().unwrap().rules.preparation_s = 60;
    let mut pending = Battle::new(&setup, 11);
    buy(&mut pending, "blue-1", "test_tank");
    pending.step();
    assert!(pending.observe(Side::Blue).own.is_empty());
    assert!(!seat(&pending, "blue-1").pending.is_empty());
    let before = (pending.digest(), pending.replay());
    assert_eq!(pending.reset_seat("blue-1", true), Err(SeatError::Occupied));
    assert_eq!((pending.digest(), pending.replay()), before);
    let mut occupied = Battle::new(&crate::seat_hand_over::setup(), 11);
    let unit = entered(&mut occupied, "blue-1");
    assert!(seat(&occupied, "blue-1").pending.is_empty());
    let before = (occupied.digest(), occupied.replay());
    assert_eq!(
        occupied.reset_seat("blue-1", false),
        Err(SeatError::Occupied)
    );
    assert_eq!((occupied.digest(), occupied.replay()), before);
    assert!(occupied.unit(unit).unwrap().alive());
}

#[test]
fn a_humans_first_order_continues_the_ai_sequence_and_drives_in_by_its_sides_road() {
    let mut setup = setup();
    setup.players[0].controller = ControllerKind::ActiveAi;
    let mut battle = Battle::new(&setup, 12);
    for _ in 0..5 * battle.rules().tick_hz {
        battle.step();
    }
    assert!(battle
        .replay()
        .accepted
        .iter()
        .any(|(_, c)| c.player.as_deref() == Some("blue-1")));
    assert!(battle
        .observe(Side::Blue)
        .own
        .iter()
        .any(|u| u.owner == Some(contract::players::PlayerIndex(0))));
    battle.hand_over("blue-1").unwrap();
    battle.step();
    battle.set_controllers(vec![("blue-1".into(), ControllerKind::Human)]);
    battle.reset_seat("blue-1", true).unwrap();
    battle.step();
    let previous = battle
        .replay()
        .accepted
        .iter()
        .filter(|(_, c)| c.player.as_deref() == Some("blue-1"))
        .map(|(_, c)| c.seq)
        .max()
        .unwrap();
    let ack = order(
        &mut battle,
        "blue-1",
        Order::ConfirmPurchase {
            variant: "test_tank".into(),
            destination: [400.0, 100.0],
        },
    );
    assert_eq!(ack.error, None);
    assert_eq!(ack.seq, previous + 1);
    for _ in 0..3000 {
        battle.step();
        if let Some(unit) = battle
            .observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.owner == Some(contract::players::PlayerIndex(0)))
        {
            let speed = battle
                .rules()
                .catalog
                .get(unit.kind)
                .mobility
                .speeds_mps()
                .0;
            assert!(setup
                .skirmish
                .as_ref()
                .unwrap()
                .sites
                .entries
                .iter()
                .filter(|e| e.side == Side::Blue)
                .any(
                    |e| (unit.position[0] - e.center[0]).hypot(unit.position[1] - e.center[1])
                        <= speed / f64::from(battle.rules().tick_hz) + 1.0
                ));
            assert!(seat(&battle, "blue-1").pending.is_empty());
            return;
        }
    }
    panic!("the first post-reset order never entered");
}

#[test]
fn reset_refuses_an_unknown_player_and_live_input_during_replay() {
    let mut live = Battle::new(&setup(), 13);
    let before = (live.digest(), live.replay());
    assert_eq!(
        live.reset_seat("stranger", true),
        Err(SeatError::MissingPlayer)
    );
    assert_eq!((live.digest(), live.replay()), before);
    let mut replay = Battle::from_replay(&setup(), &live.replay()).unwrap();
    let before = (replay.digest(), replay.replay());
    assert_eq!(
        replay.reset_seat("blue-1", true),
        Err(SeatError::ReplayInProgress)
    );
    assert_eq!((replay.digest(), replay.replay()), before);
}

#[test]
fn both_seat_events_replay_every_tick_and_a_changed_reset_does_not() {
    let setup = setup();
    let mut live = Battle::new(&setup, 14);
    buy(&mut live, "blue-1", "test_tank");
    let mut digests = Vec::new();
    for tick in 1..=180 {
        if tick == 41 {
            live.hand_over("blue-1").unwrap();
        }
        if tick == 42 {
            live.reset_seat("blue-1", true).unwrap();
        }
        if tick == 43 {
            buy(&mut live, "blue-1", "test_tank");
        }
        live.step();
        digests.push(live.digest());
    }
    assert!(live
        .observe(Side::Blue)
        .own
        .iter()
        .any(|u| u.owner == Some(contract::players::PlayerIndex(0))));
    let record = live.replay();
    assert_eq!(
        record.seat_events[1],
        SeatEvent::Reset {
            tick: 42,
            player: "blue-1".into(),
            opening: true
        }
    );
    let mut replay = Battle::from_replay(&setup, &record).unwrap();
    for digest in &digests {
        replay.step();
        assert_eq!(replay.digest(), *digest, "tick {}", replay.tick());
    }
    for event in [
        SeatEvent::Reset {
            tick: 43,
            player: "blue-1".into(),
            opening: true,
        },
        SeatEvent::Reset {
            tick: 42,
            player: "blue-2".into(),
            opening: true,
        },
        SeatEvent::Reset {
            tick: 42,
            player: "blue-1".into(),
            opening: false,
        },
    ] {
        let mut tampered = record.clone();
        tampered.seat_events[1] = event;
        let mut replay = Battle::from_replay(&setup, &tampered).unwrap();
        let mut different = false;
        for digest in &digests {
            replay.step();
            different |= replay.digest() != *digest;
        }
        assert!(different, "a changed reset reproduced every tick");
    }
}

#[test]
fn queued_hand_over_and_reset_take_one_boundary_and_replay_every_tick() {
    let setup = setup();
    let mut live = Battle::new(&setup, 41);
    entered(&mut live, "blue-1");
    let before = live.tick();
    live.hand_over("blue-1").unwrap();
    live.reset_seat("blue-1", true).unwrap();
    live.set_controllers(vec![("blue-1".into(), ControllerKind::Human)]);
    live.step();
    assert_eq!(live.tick(), before + 1);
    assert_eq!(seat(&live, "blue-1").unit_count, 0);
    assert!(seat(&live, "blue-1").pending.is_empty());
    let rules = &setup.skirmish.as_ref().unwrap().rules;
    assert_eq!(
        seat(&live, "blue-1").credits,
        f64::from(rules.credits_per_minute) * f64::from(rules.starting_minutes)
    );
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    while replay.tick() < live.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), live.digest());
}

#[test]
fn referee_reads_both_sides_without_mutating_or_counting_pending_as_living() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 42);
    let unit = entered(&mut battle, "blue-1");
    buy(&mut battle, "blue-1", "test_tank");
    let before = (battle.digest(), battle.replay());
    let view = battle.referee().unwrap();
    assert_eq!((battle.digest(), battle.replay()), before);
    let rules = &setup.skirmish.as_ref().unwrap().rules;
    assert_eq!(view.target, rules.victory_score);
    assert_eq!(view.tick, battle.tick());
    for side in [Side::Blue, Side::Red] {
        let observed = battle.observe(side);
        let observed = observed.skirmish.as_ref().unwrap();
        assert_eq!(view.scores, observed.scores);
        assert_eq!(view.phase, observed.phase);
        assert_eq!(view.result, observed.result);
        for player in &observed.players {
            let seat = view.seats.iter().find(|s| s.player == player.id).unwrap();
            assert_eq!(seat.controller, player.controller);
            assert_eq!(seat.side, side);
        }
    }
    assert!(battle.unit(unit).unwrap().alive());
    assert_eq!(view.seats[0].living, 1);
    assert_eq!(view.seats[1].living, 0);
}

#[test]
fn queued_reset_still_refuses_an_army_if_the_hand_over_loses_its_receiver() {
    let setup = setup();
    let mut live = Battle::new(&setup, 44);
    let unit = entered(&mut live, "blue-1");
    live.hand_over("blue-1").unwrap();
    live.reset_seat("blue-1", false).unwrap();
    live.set_controllers(vec![("blue-2".into(), ControllerKind::Human)]);
    live.step();
    assert!(live.unit(unit).unwrap().alive());
    assert_eq!(live.unit(unit).unwrap().owner.as_deref(), Some("blue-1"));
    assert!(seat(&live, "blue-1").credits > 0.0);
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    while replay.tick() < live.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), live.digest());
}
