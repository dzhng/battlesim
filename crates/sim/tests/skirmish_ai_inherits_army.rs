//! Does a seat's AI cope with an army it did not buy? (Multiplayer slice 20.)
//! A human-held seat with an army in a fight is handed to its AI, beside a
//! control where nobody commands it; an army shrinks while a human holds the
//! seat; and an army far over the size a seat can buy meets the order limit.
use contract::command::{Order, OrderError, RoutePolicy};
use contract::ids::{Side, Tick, UnitId};
use contract::players::{ControllerKind, PlayerIndex};
use sim::battle::Battle;
use std::collections::{BTreeMap, BTreeSet};

const SETUP_FIGHT_S: u32 = 90;
const OBSERVATION_S: u32 = 60;
const EMERGENCY_AI_S: u32 = 15;
const EMERGENCY_HUMAN_S: u32 = 60;
const EMERGENCY_RETURN_S: u32 = 30;
const BLUE_PLAYER: &str = "blue-1";

fn setup_for_army(size: usize) -> contract::scenario::ScenarioDefinition {
    let mut setup = crate::skirmish::setup();
    setup.players[0].controller = ControllerKind::Human;
    // Red is a real opponent from tick zero. Its wallet, cap and purchase
    // rules are the same as Blue's; only Blue's seat changes controller.
    setup.players[1].controller = ControllerKind::ActiveAi;
    // The test deck sells test_tank to the US faction only.
    setup.players[1].faction = Some(contract::catalog::Faction::Us);
    setup.map.size = [1_200.0, 800.0];
    let skirmish = setup.skirmish.as_mut().unwrap();
    // The two sides enter out of weapon range of each other, then drive to
    // the same reachable flags.
    skirmish.sites.entries = vec![
        contract::encounter::EntrySite {
            side: Side::Blue,
            center: [10.0, 10.0],
            yaw: std::f64::consts::FRAC_PI_2,
        },
        contract::encounter::EntrySite {
            side: Side::Red,
            center: [1_190.0, 790.0],
            yaw: -std::f64::consts::FRAC_PI_2,
        },
    ];
    skirmish.rules.preparation_s = 0;
    skirmish.rules.max_units = size as u32;
    // test_tank costs 200: the smallest starting wallet that buys `size`.
    skirmish.rules.credits_per_minute = 200;
    skirmish.rules.starting_minutes = size as u32;
    // The fight must outlast the observation window; combat, capture and
    // income are as shipped.
    skirmish.rules.majority_s = 1_000_000;
    skirmish.rules.victory_score = 1_000_000_000_000.0;
    setup
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    tick: Tick,
    blue_objectives: Vec<(String, Option<Side>)>,
    blue_live: usize,
    red_live: usize,
    blue_score: f64,
    red_score: f64,
    phase: contract::skirmish::Phase,
    result: Option<contract::skirmish::MatchResult>,
}

fn snapshot(battle: &Battle) -> Snapshot {
    let view = battle.observe(Side::Blue).skirmish.as_ref().unwrap();
    Snapshot {
        tick: battle.tick(),
        blue_objectives: view
            .objectives
            .iter()
            .map(|objective| (objective.id.clone(), objective.owner))
            .collect(),
        blue_live: battle.observe(Side::Blue).own.len(),
        red_live: battle.observe(Side::Red).own.len(),
        blue_score: view.scores[Side::Blue.index()],
        red_score: view.scores[Side::Red.index()],
        phase: view.phase,
        result: view.result,
    }
}

fn assemble_army(battle: &mut Battle, size: usize) {
    for index in 0..size {
        let ack = battle.accept_order(
            Side::Blue,
            Some(BLUE_PLAYER.into()),
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [[300.0, 180.0], [600.0, 180.0], [900.0, 180.0]][index % 3],
            },
            false,
        );
        assert_eq!(ack.error, None, "purchase {index} was refused: {ack:?}");
        battle.step();
        // Every twenty purchases, move what has entered off the entry road.
        if (index + 1) % 20 == 0 {
            let units = battle
                .observe(Side::Blue)
                .own
                .iter()
                .map(|unit| unit.id)
                .collect();
            let clear = battle.accept_order(
                Side::Blue,
                Some(BLUE_PLAYER.into()),
                Order::Move {
                    units,
                    gesture: 10_000 + index as u64,
                    goal: [1_000.0, 600.0],
                    route: RoutePolicy::Fastest,
                    direction: Default::default(),
                    facing: None,
                },
                false,
            );
            assert_eq!(clear.error, None, "the move was refused: {clear:?}");
            for _ in 0..battle.rules().tick_hz {
                battle.step();
            }
        }
    }
    let deadline = battle.tick() + (240 * battle.rules().tick_hz as u64);
    while battle.tick() < deadline {
        if battle
            .observe(Side::Blue)
            .own
            .iter()
            .filter(|unit| unit.owner == Some(PlayerIndex(0)))
            .count()
            == size
        {
            return;
        }
        battle.step();
    }
    panic!(
        "army assembled only {} of {size} units after {} ticks; status={:?}",
        battle
            .observe(Side::Blue)
            .own
            .iter()
            .filter(|unit| unit.owner == Some(PlayerIndex(0)))
            .count(),
        deadline,
        battle
            .observe(Side::Blue)
            .skirmish
            .as_ref()
            .map(|view| (&view.phase, &view.players))
    );
}

/// The human makes the inherited army a fighting army before T, exactly once
/// in both arms of the comparison: buy the cards, then send every survivor to
/// the centre objective with one ordinary player move.
fn prepare_fight(size: usize, seed: u64) -> Battle {
    let mut battle = Battle::new(&setup_for_army(size), seed);
    assemble_army(&mut battle, size);
    let units = battle
        .observe(Side::Blue)
        .own
        .iter()
        .filter(|unit| unit.owner == Some(PlayerIndex(0)))
        .map(|unit| unit.id)
        .collect();
    let ack = battle.accept_order(
        Side::Blue,
        Some(BLUE_PLAYER.into()),
        Order::Move {
            units,
            gesture: 1,
            goal: [200.0, 200.0],
            route: RoutePolicy::Fastest,
            direction: Default::default(),
            facing: None,
        },
        false,
    );
    assert_eq!(ack.error, None, "human attack move was refused: {ack:?}");
    for _ in 0..SETUP_FIGHT_S * battle.rules().tick_hz {
        battle.step();
    }
    battle
}

#[derive(Debug, Default)]
struct NoGoalDiagnostic {
    max_streak: Tick,
    unit: Option<UnitId>,
    tick: Option<Tick>,
    position: Option<[f64; 3]>,
    last_order: Option<String>,
}

#[derive(Debug, Default)]
struct OrderMetrics {
    refresh_ticks: Tick,
    applied_by_refresh: Vec<(usize, usize)>,
    rate_refused_by_refresh: Vec<(usize, usize)>,
    first_applied_delay: BTreeMap<UnitId, Option<Tick>>,
    longest_refused_streak: BTreeMap<UnitId, Tick>,
    no_goal: NoGoalDiagnostic,
}

#[derive(Debug)]
struct RunMetrics {
    seed: u64,
    size: usize,
    handover: bool,
    at_t: Snapshot,
    at_n: Snapshot,
    orders: OrderMetrics,
    rate_refused: usize,
    other_refused: usize,
}

fn order_units(order: &Order) -> Vec<UnitId> {
    match order {
        Order::Refund { units }
        | Order::Move { units, .. }
        | Order::Stop { units }
        | Order::Attack { units, .. }
        | Order::AttackMove { units, .. }
        | Order::SetEngagement { units, .. }
        | Order::SetAfterburner { units, .. }
        | Order::SetDeployment { units, .. }
        | Order::Garrison { units, .. }
        | Order::OccupyBuilding { units, .. }
        | Order::ExitBuilding { units }
        | Order::Board { units, .. }
        | Order::Collect { units, .. }
        | Order::Unload { units, .. }
        | Order::PopSmoke { units } => units.clone(),
        Order::Ready
        | Order::CancelPending { .. }
        | Order::ConfirmPurchase { .. }
        | Order::UpgradeMove { .. } => Vec::new(),
    }
}

fn collect_order_metrics(
    battle: &Battle,
    takeover_tick: Tick,
    held: &BTreeSet<UnitId>,
    duration_ticks: Tick,
    mut no_goal: NoGoalDiagnostic,
) -> (OrderMetrics, usize, usize) {
    let refresh_ticks =
        (battle.team_tuning().planner_decision_s * f64::from(battle.rules().tick_hz)) as Tick;
    let commands: Vec<_> = battle
        .replay()
        .accepted
        .into_iter()
        .filter(|(tick, command)| {
            *tick > takeover_tick && command.player.as_deref() == Some(BLUE_PLAYER)
        })
        .collect();
    let receipts: Vec<_> = battle
        .ai_receipts()
        .iter()
        .filter(|receipt| receipt.tick >= takeover_tick && receipt.player == BLUE_PLAYER)
        .collect();
    assert_eq!(
        commands.len(),
        receipts.len(),
        "AI receipt and replay streams diverged"
    );

    let mut metrics = OrderMetrics {
        refresh_ticks,
        first_applied_delay: held.iter().map(|&id| (id, None)).collect(),
        ..Default::default()
    };
    let mut refused_buckets: BTreeMap<UnitId, BTreeSet<Tick>> = BTreeMap::new();
    let mut applied = BTreeMap::<Tick, usize>::new();
    let mut rate_refused = BTreeMap::<Tick, usize>::new();
    let mut rate_count = 0;
    let mut other_count = 0;

    for ((applied_tick, command), receipt) in commands.into_iter().zip(receipts) {
        let bucket = (receipt.tick.saturating_sub(takeover_tick)) / refresh_ticks.max(1);
        let units = order_units(&command.order);
        if receipt.accepted {
            *applied.entry(bucket).or_default() += 1;
            for unit in &units {
                if !held.contains(unit) {
                    continue;
                }
                let delay = receipt.tick.saturating_sub(takeover_tick);
                match metrics.first_applied_delay.get_mut(unit) {
                    Some(first) if first.is_none() => *first = Some(delay),
                    Some(_) => {}
                    None => {
                        metrics.first_applied_delay.insert(*unit, Some(delay));
                    }
                }
            }
        } else if receipt.error == Some(OrderError::RateLimited) {
            rate_count += 1;
            *rate_refused.entry(bucket).or_default() += 1;
            for unit in units {
                if !held.contains(&unit) {
                    continue;
                }
                refused_buckets.entry(unit).or_default().insert(bucket);
            }
        } else {
            other_count += 1;
        }
        assert_eq!(
            applied_tick,
            receipt.tick + 1,
            "AI order receipt has an unexpected application tick"
        );
    }

    if let (Some(unit), Some(tick)) = (no_goal.unit, no_goal.tick) {
        for ((_, command), receipt) in
            battle
                .replay()
                .accepted
                .into_iter()
                .filter(|(tick, command)| {
                    *tick > takeover_tick && command.player.as_deref() == Some(BLUE_PLAYER)
                })
                .zip(battle.ai_receipts().iter().filter(|receipt| {
                    receipt.tick >= takeover_tick && receipt.player == BLUE_PLAYER
                }))
        {
            if receipt.accepted
                && receipt.tick <= tick
                && order_units(&command.order).contains(&unit)
            {
                no_goal.last_order = Some(format!("{:?}", command.order));
            }
        }
    }

    let bucket_count = (duration_ticks / refresh_ticks.max(1)) as usize + 1;
    metrics.applied_by_refresh = (0..bucket_count)
        .map(|bucket| (bucket, applied.get(&(bucket as Tick)).copied().unwrap_or(0)))
        .collect();
    metrics.rate_refused_by_refresh = (0..bucket_count)
        .map(|bucket| {
            (
                bucket,
                rate_refused.get(&(bucket as Tick)).copied().unwrap_or(0),
            )
        })
        .collect();
    for (unit, buckets) in refused_buckets {
        let mut longest = 0;
        let mut current = 0;
        let mut previous = None;
        for bucket in buckets {
            if previous.is_some_and(|old| bucket == old + 1) {
                current += 1;
            } else {
                current = 1;
            }
            longest = longest.max(current);
            previous = Some(bucket);
        }
        metrics.longest_refused_streak.insert(unit, longest);
    }
    metrics.no_goal = no_goal;
    (metrics, rate_count, other_count)
}

fn run_once(size: usize, seed: u64, handover: bool) -> RunMetrics {
    let mut battle = prepare_fight(size, seed);
    let at_t = snapshot(&battle);
    assert!(
        at_t.blue_objectives
            .iter()
            .any(|(_, owner)| *owner == Some(Side::Blue)),
        "Blue held no objective at handover tick {at_t:?}; units={:?}",
        battle
            .observe(Side::Blue)
            .own
            .iter()
            .take(3)
            .map(|unit| (unit.position, unit.goal))
            .collect::<Vec<_>>()
    );
    let held: BTreeSet<_> = battle
        .observe(Side::Blue)
        .own
        .iter()
        .filter(|unit| unit.owner == Some(PlayerIndex(0)))
        .map(|unit| unit.id)
        .collect();
    let takeover_tick = battle.tick();
    if handover {
        battle.set_controllers(vec![(BLUE_PLAYER.into(), ControllerKind::ActiveAi)]);
    }
    let duration_ticks = OBSERVATION_S * battle.rules().tick_hz;
    let mut no_goal_streak: BTreeMap<UnitId, Tick> = held.iter().map(|&id| (id, 0)).collect();
    let mut no_goal = NoGoalDiagnostic::default();
    for _ in 0..duration_ticks {
        battle.step();
        for unit in battle
            .observe(Side::Blue)
            .own
            .iter()
            .filter(|unit| held.contains(&unit.id))
        {
            let streak = no_goal_streak.entry(unit.id).or_default();
            if unit.goal.is_none() {
                *streak += 1;
                if *streak > no_goal.max_streak {
                    no_goal.max_streak = *streak;
                    no_goal.unit = Some(unit.id);
                    no_goal.tick = Some(battle.tick());
                    no_goal.position = Some(unit.position);
                }
            } else {
                *streak = 0;
            }
        }
    }
    let at_n = snapshot(&battle);
    let (orders, rate_refused, other_refused) = collect_order_metrics(
        &battle,
        takeover_tick,
        &held,
        duration_ticks as Tick,
        no_goal,
    );
    RunMetrics {
        seed,
        size,
        handover,
        at_t,
        at_n,
        orders,
        rate_refused,
        other_refused,
    }
}

fn held_objectives(snapshot: &Snapshot) -> BTreeSet<String> {
    snapshot
        .blue_objectives
        .iter()
        .filter(|(_, owner)| *owner == Some(Side::Blue))
        .map(|(id, _)| id.clone())
        .collect()
}

fn assert_retained(metrics: &RunMetrics) {
    for objective in held_objectives(&metrics.at_t) {
        assert!(
            held_objectives(&metrics.at_n).contains(&objective),
            "seed {} size {} lost {objective} after handover: {:?}",
            metrics.seed,
            metrics.size,
            metrics.at_n
        );
    }
}

fn report_run(metrics: &RunMetrics) {
    eprintln!(
        "seed={} size={} arm={} T={:?} N={:?} applied/refused={:?}/{:?} rate_refused={} other_refused={} first_applied={:?} refused_streak={:?}",
        metrics.seed,
        metrics.size,
        if metrics.handover { "handover" } else { "control" },
        metrics.at_t,
        metrics.at_n,
        metrics.orders.applied_by_refresh,
        metrics.orders.rate_refused_by_refresh,
        metrics.rate_refused,
        metrics.other_refused,
        metrics.orders.first_applied_delay,
        metrics.orders.longest_refused_streak,
    );
    eprintln!("  no-goal diagnostic: {:?}", metrics.orders.no_goal);
}

fn assert_ordering(metrics: &RunMetrics) {
    for (unit, delay) in &metrics.orders.first_applied_delay {
        assert!(
            delay.is_some_and(|ticks| ticks <= metrics.orders.refresh_ticks),
            "seed {} size {} unit {unit:?} had no applied order within one planner refresh: {delay:?}",
            metrics.seed,
            metrics.size
        );
    }
    assert!(
        metrics
            .orders
            .longest_refused_streak
            .values()
            .all(|streak| *streak <= 1),
        "seed {} size {} has repeated rate-refused refreshes: {:?}",
        metrics.seed,
        metrics.size,
        metrics.orders.longest_refused_streak
    );
}

#[test]
fn a_handed_thirty_unit_fighting_army_beats_the_same_seed_control() {
    // The control sends no Blue orders after the handover. Seeds 31 to 35
    // were run and are tabled in the slice; the one where the handed army
    // fared worst stands guard.
    let seed = 33;
    let handover = run_once(30, seed, true);
    let control = run_once(30, seed, false);
    report_run(&handover);
    report_run(&control);
    assert_retained(&handover);
    assert_ordering(&handover);
    assert!(
        handover.at_n.blue_score >= control.at_n.blue_score,
        "the handed army scored below the one nobody commanded: {:?} vs {:?}",
        handover.at_n,
        control.at_n
    );
}

/// A seat that can field `size` units on three roads, against a side that
/// fields nothing: the order limit is the question here, not the fight.
fn setup_for_parade(size: usize) -> contract::scenario::ScenarioDefinition {
    let mut setup = crate::skirmish::setup();
    setup.players[0].controller = ControllerKind::Human;
    setup.players[1].controller = ControllerKind::PassiveAi;
    for index in 2..=3 {
        let mut blue = setup.players[0].clone();
        blue.id = format!("blue-{index}");
        blue.controller = ControllerKind::PassiveAi;
        let mut red = setup.players[1].clone();
        red.id = format!("red-{index}");
        setup.players.extend([blue, red]);
    }
    let skirmish = setup.skirmish.as_mut().unwrap();
    skirmish.sites.entries = [150.0, 400.0, 650.0]
        .into_iter()
        .flat_map(|x| {
            [
                contract::encounter::EntrySite {
                    side: Side::Blue,
                    center: [x, 10.0],
                    yaw: std::f64::consts::FRAC_PI_2,
                },
                contract::encounter::EntrySite {
                    side: Side::Red,
                    center: [x, 590.0],
                    yaw: -std::f64::consts::FRAC_PI_2,
                },
            ]
        })
        .collect();
    skirmish.rules.preparation_s = 0;
    skirmish.rules.max_units = size as u32;
    skirmish.rules.credits_per_minute = 200;
    skirmish.rules.starting_minutes = size as u32;
    skirmish.rules.majority_s = 1_000_000;
    skirmish.rules.victory_score = 1_000_000_000_000.0;
    setup
}

/// Buy `size` units for the human seat, one order at a time at the rate the
/// battle allows, and wait until they have all entered.
fn parade(size: usize, seed: u64) -> Battle {
    let mut battle = Battle::new(&setup_for_parade(size), seed);
    let hz = battle.rules().tick_hz;
    let between = (hz / battle.rules().commands.orders_per_s).max(1);
    for index in 0..size {
        let ack = battle.accept_order(
            Side::Blue,
            Some(BLUE_PLAYER.into()),
            Order::ConfirmPurchase {
                variant: "test_tank".into(),
                destination: [[150.0, 300.0], [400.0, 300.0], [650.0, 300.0]][index % 3],
            },
            false,
        );
        assert_eq!(ack.error, None, "purchase {index} was refused: {ack:?}");
        for _ in 0..between {
            battle.step();
        }
    }
    let deadline = battle.tick() + u64::from((size as u32 / 3 + 180) * hz);
    while battle.observe(Side::Blue).own.len() < size {
        assert!(
            battle.tick() < deadline,
            "only {} of {size} units entered",
            battle.observe(Side::Blue).own.len()
        );
        battle.step();
    }
    battle
}

/// Hand a seat holding `size` units to its AI and say what became of the
/// orders it gave over the next minute.
fn orders_after_handover(size: usize, seed: u64) -> (OrderMetrics, usize, usize) {
    let mut battle = parade(size, seed);
    let held: BTreeSet<_> = battle
        .observe(Side::Blue)
        .own
        .iter()
        .map(|unit| unit.id)
        .collect();
    let takeover_tick = battle.tick();
    battle.set_controllers(vec![(BLUE_PLAYER.into(), ControllerKind::ActiveAi)]);
    let duration_ticks = OBSERVATION_S * battle.rules().tick_hz;
    for _ in 0..duration_ticks {
        battle.step();
    }
    let (orders, rate_refused, other_refused) = collect_order_metrics(
        &battle,
        takeover_tick,
        &held,
        Tick::from(duration_ticks),
        NoGoalDiagnostic::default(),
    );
    let ordered_late = orders
        .first_applied_delay
        .values()
        .filter(|delay| delay.is_none_or(|ticks| ticks > orders.refresh_ticks))
        .count();
    let never = orders
        .first_applied_delay
        .values()
        .filter(|delay| delay.is_none())
        .count();
    eprintln!(
        "size={size} seed={seed} applied/refresh={:?} rate_refused/refresh={:?} rate_refused={rate_refused} other_refused={other_refused} units_first_ordered_after_one_refresh={ordered_late} never_ordered={never} units_refused_in_consecutive_refreshes={:?}",
        orders.applied_by_refresh,
        orders.rate_refused_by_refresh,
        orders
            .longest_refused_streak
            .values()
            .filter(|streak| **streak > 1)
            .count(),
    );
    (orders, rate_refused, other_refused)
}

#[test]
#[ignore = "fails, and that is the finding: the AI gives its orders in one burst each planner round and the order limit lets 30 through, so 150 units take five rounds (25 s) to all be ordered; 288 orders refused"]
fn every_unit_of_an_inherited_army_of_one_hundred_fifty_gets_its_orders() {
    let (orders, rate_refused, _) = orders_after_handover(150, 351);
    assert_eq!(rate_refused, 0, "the AI's orders ran into the order limit");
    for (unit, delay) in &orders.first_applied_delay {
        assert!(
            delay.is_some_and(|ticks| ticks <= orders.refresh_ticks),
            "unit {unit:?} had no applied order within one planner refresh: {delay:?}"
        );
    }
}

#[test]
fn handing_back_a_shrunken_army_does_not_abandon_its_objective_for_emergency_defence() {
    let mut battle = prepare_fight(30, 401);
    battle.set_controllers(vec![(BLUE_PLAYER.into(), ControllerKind::ActiveAi)]);
    for _ in 0..EMERGENCY_AI_S * battle.rules().tick_hz {
        battle.step();
    }
    let before_human = snapshot(&battle);
    let pivots_before = battle.ai_pivots_by_side()[Side::Blue.index()];
    let receipts_before = battle
        .ai_receipts()
        .iter()
        .filter(|receipt| receipt.player == BLUE_PLAYER)
        .count();
    battle.set_controllers(vec![(BLUE_PLAYER.into(), ControllerKind::Human)]);
    for _ in 0..EMERGENCY_HUMAN_S * battle.rules().tick_hz {
        battle.step();
    }
    let during_human = snapshot(&battle);
    let pivots_during_human = battle.ai_pivots_by_side()[Side::Blue.index()];
    let receipts_during_human = battle
        .ai_receipts()
        .iter()
        .filter(|receipt| receipt.player == BLUE_PLAYER)
        .count();
    battle.set_controllers(vec![(BLUE_PLAYER.into(), ControllerKind::ActiveAi)]);
    for _ in 0..EMERGENCY_RETURN_S * battle.rules().tick_hz {
        battle.step();
    }
    let after = snapshot(&battle);
    let pivots_after = battle.ai_pivots_by_side()[Side::Blue.index()];
    eprintln!(
        "emergency seed=401 before_human={before_human:?} during_human={during_human:?} after={after:?} pivots={pivots_before}->{pivots_during_human}->{pivots_after} receipts={receipts_before}->{receipts_during_human}"
    );
    assert!(
        during_human.blue_live < before_human.blue_live,
        "the human interval did not shrink the army: {before_human:?} -> {during_human:?}"
    );
    assert_eq!(
        pivots_during_human, pivots_before,
        "AI doctrine kept running while the seat was human"
    );
    assert_eq!(
        receipts_during_human, receipts_before,
        "AI issued orders while the seat was human"
    );
    for objective in held_objectives(&before_human) {
        assert!(
            held_objectives(&after).contains(&objective),
            "army loss changed held objective {objective}: {before_human:?} -> {after:?}"
        );
    }
}
