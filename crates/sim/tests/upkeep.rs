//! Upkeep: a player's passive income tapers as the army it fields grows.
use crate::common::command;
use contract::command::Order;
use contract::ids::Side;
use contract::skirmish::PlayerView;
use serde_json::json;
use sim::battle::Battle;

/// A duel plus an idle second Blue player, with income tapering between
/// one and four test tanks' worth of army.
fn setup() -> (contract::scenario::ScenarioDefinition, f64, f64) {
    let mut setup = crate::skirmish::setup();
    let mut blue_two = setup.players[0].clone();
    blue_two.id = "blue-2".into();
    setup.players.push(blue_two);
    let entries = &mut setup.skirmish.as_mut().unwrap().sites.entries;
    let mut second = entries.clone();
    for entry in &mut second {
        entry.center[0] = 700.0;
    }
    entries.extend(second);
    let catalog = &setup.rules.catalog;
    let tank = f64::from(catalog.get(catalog.index("test_tank").unwrap()).cost);
    let rules = &mut setup.skirmish.as_mut().unwrap().rules;
    rules.upkeep = Some(
        serde_json::from_value(json!({
            "taper_from": tank as u32,
            "taper_to": 4 * tank as u32,
            "floor_percent": 25
        }))
        .unwrap(),
    );
    let full = f64::from(rules.credits_per_minute);
    (setup, full, full * 0.25)
}

fn player<'a>(battle: &'a Battle, side: Side, id: &str) -> &'a PlayerView {
    let frame = battle.observe(side);
    frame
        .skirmish
        .as_ref()
        .unwrap()
        .players
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("{id} is not on {side:?}'s view"))
}

fn rate(battle: &Battle, side: Side, id: &str) -> f64 {
    player(battle, side, id).income_per_minute
}

#[test]
fn income_tapers_with_the_fielded_army_per_player_and_recovers_as_it_leaves() {
    let (setup, full, floor) = setup();
    let mut battle = Battle::new(&setup, 1);
    let mut spot = 0.0;
    let mut near_base = || {
        spot += 1.0;
        Order::purchase("test_tank", [220.0 + 60.0 * spot, 120.0])
    };
    let mut seq = [0; 2];
    let mut order = |battle: &mut Battle, side: Side, order: Order| {
        seq[side.index()] += 1;
        let ack = battle.accept(command(side, seq[side.index()], order));
        assert!(ack.error.is_none(), "{ack:?}");
    };
    battle.step();
    assert_eq!(
        rate(&battle, Side::Blue, "blue-1"),
        full,
        "no army, full income"
    );

    // Two tanks still on their way already cost upkeep, part way down the taper.
    for _ in 0..2 {
        order(&mut battle, Side::Blue, near_base());
    }
    battle.step();
    let two = rate(&battle, Side::Blue, "blue-1");
    assert!(floor < two && two < full, "two incoming tanks: {two}");
    assert_eq!(
        rate(&battle, Side::Blue, "blue-2"),
        full,
        "an idle teammate keeps full income"
    );
    assert_eq!(
        rate(&battle, Side::Red, "red-1"),
        full,
        "the enemy's army is not ours"
    );

    // A bigger army costs more, down to the floor and no further.
    order(&mut battle, Side::Blue, near_base());
    battle.step();
    let three = rate(&battle, Side::Blue, "blue-1");
    assert!(
        floor < three && three < two,
        "three tanks: {three}, two: {two}"
    );
    for _ in 0..2 {
        order(&mut battle, Side::Blue, near_base());
    }
    battle.step();
    assert_eq!(
        rate(&battle, Side::Blue, "blue-1"),
        floor,
        "five tanks: past the taper"
    );

    // Cancelling a purchase lifts upkeep at once.
    let last = player(&battle, Side::Blue, "blue-1")
        .pending
        .last()
        .unwrap()
        .id;
    order(
        &mut battle,
        Side::Blue,
        Order::CancelPending { purchase: last },
    );
    battle.step();
    assert_eq!(
        rate(&battle, Side::Blue, "blue-1"),
        floor,
        "four tanks: the taper's end"
    );

    // The wallet grows at the published rate once the battle runs, and a
    // purchase arriving on the map changes nothing.
    let hz = battle.rules().tick_hz;
    while battle.observe(Side::Blue).skirmish.as_ref().unwrap().phase
        != contract::skirmish::Phase::Active
    {
        battle.step();
    }
    let wallets = |battle: &Battle| {
        [
            player(battle, Side::Blue, "blue-1").credits,
            player(battle, Side::Blue, "blue-2").credits,
            player(battle, Side::Red, "red-1").credits,
        ]
    };
    let before = wallets(&battle);
    for _ in 0..60 * hz {
        battle.step();
    }
    let after = wallets(&battle);
    for (index, expected) in [floor, full, full].into_iter().enumerate() {
        let earned = after[index] - before[index];
        assert!(
            (earned - expected).abs() < 1e-6,
            "player {index} earned {earned} in a minute, expected {expected}"
        );
    }

    for _ in 0..120 * hz {
        if battle.observe(Side::Blue).own.len() == 4 {
            break;
        }
        battle.step();
    }
    assert_eq!(battle.observe(Side::Blue).own.len(), 4, "the tanks arrived");
    assert_eq!(rate(&battle, Side::Blue, "blue-1"), floor);

    // Sending three home relieves upkeep once they leave the map.
    let home: Vec<_> = battle.observe(Side::Blue).own[..3]
        .iter()
        .map(|u| u.id)
        .collect();
    order(&mut battle, Side::Blue, Order::Refund { units: home });
    for _ in 0..60 * hz {
        if battle.observe(Side::Blue).own.len() == 1 {
            break;
        }
        battle.step();
    }
    assert_eq!(
        battle.observe(Side::Blue).own.len(),
        1,
        "three tanks retired"
    );
    assert_eq!(
        rate(&battle, Side::Blue, "blue-1"),
        full,
        "one tank costs no upkeep"
    );

    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    while replay.tick() < battle.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), battle.digest());
}
