use super::inputs::{prepare_row, Inputs};
use super::model::{ProgressSignature, Report, Row, Status};
use contract::catalog::Faction;
use contract::command::Order;
use contract::ids::Side;
use contract::scenario::Rules;
use sim::skirmish_ai::team::{cohesion_sample, group_ready};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) fn run_safe(inputs: &Inputs, row: Row) -> Report {
    let fallback_id = row.id.clone();
    let seed = row.seed;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(inputs, row))) {
        Ok(report) => report,
        Err(panic) => {
            let mut report = empty_report(fallback_id, inputs.factions);
            report.seed = seed;
            report.diagnostic = Some(
                panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|value| (*value).into()))
                    .unwrap_or_else(|| "non-text panic".into()),
            );
            report
        }
    }
}

fn empty_report(id: String, factions: [Faction; 2]) -> Report {
    Report {
        id,
        seed: 0,
        max_battle_seconds: 0,
        battle_seconds: 0.0,
        status: Status::Refused,
        ticks: 0,
        battle_digest: 0,
        refused_orders: 0,
        refused_errors: BTreeMap::new(),
        refused_by_side: [0, 0],
        accepted_orders: 0,
        attempted_orders: 0,
        policy: "balanced_doctrine",
        factions,
        diagnostic: None,
        result: None,
        objective_first_owner_ticks: BTreeMap::new(),
        objective_owned_ticks: [BTreeMap::new(), BTreeMap::new()],
        peak_objectives_held: [0, 0],
        cohesion: BTreeMap::new(),
        first_contact_tick: None,
        recon_sightings: 0,
        doctrine_pivots: [0, 0],
        map_identity: None,
        site_count: 0,
        replay_digest: None,
        replay_match: None,
        credit_spend: [0, 0],
        opening: [vec![], vec![]],
        living_cost: [0, 0],
        recon_range_m: 0.0,
        launcher_held_s: BTreeMap::new(),
        air: None,
    }
}

pub(crate) fn run(inputs: &Inputs, row: Row) -> Report {
    let mut report = empty_report(row.id.clone(), inputs.factions);
    report.seed = row.seed;
    report.max_battle_seconds = row.horizon_ticks / u64::from(inputs.rules.tick_hz);
    let (setup, prepared, map_identity) = match prepare_row(inputs, &row) {
        Ok(value) => value,
        Err(error) => {
            report.diagnostic = Some(error);
            return report;
        }
    };
    report.map_identity = map_identity;
    report.site_count = setup
        .skirmish
        .as_ref()
        .map(|skirmish| skirmish.sites.objectives.len() as u32)
        .unwrap_or_default();
    let mut battle = sim::battle::Battle::from_prepared(&setup, row.seed, prepared);
    for side in Side::COMBATANTS {
        let Some(list) = &inputs.buy[side.index()] else {
            continue;
        };
        let purchases = list
            .iter()
            .filter(|card| card.as_str() != "|")
            .map(|card| setup.rules.catalog.index(card).expect("validated buy card"))
            .collect::<Vec<_>>();
        let player = if side == Side::Blue {
            "blue-1"
        } else {
            "red-1"
        };
        let repeat_from = list
            .iter()
            .position(|card| card == "|")
            .map(|bar| {
                list[..bar]
                    .iter()
                    .filter(|card| card.as_str() != "|")
                    .count()
            })
            .or(Some(0));
        battle.script_purchases(player, purchases, repeat_from);
    }
    let mut stagnant = 0;
    let mut previous: Option<ProgressSignature> = None;
    let mut cohesion_started = BTreeSet::new();
    while report.ticks < row.horizon_ticks {
        for plan in battle.team_plans().iter().flatten() {
            for player_plan in &plan.players {
                let Some(player) = setup.players.get(player_plan.player.0 as usize) else {
                    continue;
                };
                if battle
                    .observe(player.side)
                    .skirmish
                    .as_ref()
                    .is_none_or(|view| view.phase != contract::skirmish::Phase::Active)
                {
                    continue;
                }
                let positions = sim::skirmish_ai::team::group_positions(
                    &setup.rules,
                    battle
                        .observe(player.side)
                        .own
                        .iter()
                        .filter(|unit| unit.owner == Some(player_plan.player)),
                );
                let group_radius = sim::skirmish_ai::team::TeamTuning::shipped().group_radius_m;
                if group_ready(&positions, player_plan.staging_point, group_radius) {
                    cohesion_started.insert(player.id.clone());
                }
                if !cohesion_started.contains(&player.id) {
                    continue;
                }
                let counts = report.cohesion.entry(player.id.clone()).or_insert([0, 0]);
                let sample = cohesion_sample(&positions, player_plan.staging_point, group_radius);
                counts[0] += sample[0];
                counts[1] += sample[1];
            }
        }
        battle.step();
        report.ticks += 1;
        for side in Side::COMBATANTS {
            let frame = battle.observe(side);
            if !frame.contacts.is_empty() || !frame.identified.is_empty() {
                report.first_contact_tick.get_or_insert(frame.tick);
            }
            for unit in frame.own.iter().filter(|unit| {
                unit.mounts
                    .iter()
                    .any(|m| m.reason == contract::observation::ActionReason::BlockedTrajectory)
                    && setup.rules.catalog.mounts(unit.kind).iter().any(|mount| {
                        mount.def.weapons.iter().any(|w| {
                            setup
                                .rules
                                .weapons
                                .get(w)
                                .is_some_and(|w| w.requires_clear_launch_path)
                        })
                    })
            }) {
                *report
                    .launcher_held_s
                    .entry(setup.rules.catalog.id(unit.kind).into())
                    .or_default() += 1.0 / f64::from(setup.rules.tick_hz);
            }
            report.recon_sightings += frame
                .own
                .iter()
                .filter(|unit| {
                    !unit.sees.is_empty() && setup.rules.catalog.get(unit.kind).has_role("recon")
                })
                .map(|unit| unit.sees.len() as u32)
                .sum::<u32>();
            for unit in frame
                .own
                .iter()
                .filter(|unit| setup.rules.catalog.get(unit.kind).has_role("recon"))
            {
                for target in &frame.identified {
                    if unit.sees.contains(&target.id) {
                        let dx = unit.position[0] - target.position[0];
                        let dy = unit.position[1] - target.position[1];
                        report.recon_range_m = report.recon_range_m.max(libm::hypot(dx, dy));
                    }
                }
            }
        }
        for side in Side::COMBATANTS {
            report.living_cost[side.index()] = battle
                .observe(side)
                .own
                .iter()
                .map(|unit| setup.rules.catalog.get(unit.kind).cost as u64)
                .sum();
        }
        let view = battle.observe(Side::Blue).skirmish.as_ref();
        if let Some(view) = view {
            for objective in &view.objectives {
                if objective.owner.is_some() {
                    report
                        .objective_first_owner_ticks
                        .entry(objective.id.clone())
                        .or_insert(report.ticks);
                }
                if let Some(owner) = objective.owner {
                    if owner != Side::Neutral {
                        *report.objective_owned_ticks[owner.index()]
                            .entry(objective.id.clone())
                            .or_default() += 1;
                    }
                }
            }
            // A team takes several objectives at once (slice 12): the most
            // each side held at the same moment, whoever took them.
            for side in Side::COMBATANTS {
                let held = view
                    .objectives
                    .iter()
                    .filter(|objective| objective.owner == Some(side))
                    .count() as u64;
                report.peak_objectives_held[side.index()] =
                    report.peak_objectives_held[side.index()].max(held);
            }
        }
        let progress = view
            .map(|v| {
                let owned = v.objectives.iter().filter(|o| o.owner.is_some()).count() as u32;
                let capture = v.objectives.iter().filter(|o| o.contested).count() as u32;
                let capture_bits = v
                    .objectives
                    .iter()
                    .map(|o| o.capture_progress.to_bits())
                    .fold(0u64, u64::wrapping_add);
                let observed = Side::ALL
                    .into_iter()
                    .map(|side| {
                        let frame = battle.observe(side);
                        (frame.own.len() as u64).wrapping_add(frame.contacts.len() as u64)
                    })
                    .sum();
                let movement = Side::ALL
                    .into_iter()
                    .flat_map(|side| {
                        battle.observe(side).own.iter().map(|unit| {
                            (
                                unit.id.0,
                                unit.position[0].to_bits(),
                                unit.position[1].to_bits(),
                                unit.goal.map(|goal| (goal[0].to_bits(), goal[1].to_bits())),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                (
                    owned,
                    capture,
                    v.scores.iter().sum::<f64>().to_bits(),
                    capture_bits,
                    observed,
                    movement,
                    report.accepted_orders,
                )
            })
            .unwrap_or_default();
        // Preparation intentionally has no movement or combat progress. Do
        // not let that authored pause trip the no-progress guard on short
        // corpus probes; the guard applies once the battle is active.
        let grace_ticks = u64::from(inputs.rules.tick_hz) * 5;
        stagnant = if report.ticks < grace_ticks
            || view.is_some_and(|v| v.phase == contract::skirmish::Phase::Preparation)
        {
            0
        } else if previous.as_ref().is_some_and(|old| *old == progress) {
            stagnant + 1
        } else {
            0
        };
        previous = Some(progress);
        if view.is_some_and(|v| v.phase == contract::skirmish::Phase::Finished) {
            report.status = Status::Complete;
            break;
        }
        if stagnant >= row.no_progress_ticks {
            report.status = Status::NoProgress;
            break;
        }
    }
    if matches!(report.status, Status::Refused) && report.ticks >= row.horizon_ticks {
        report.status = Status::Timeout;
    }
    report.battle_digest = battle.digest();
    // Battle owns all policy acceptance and refusal evidence.
    let receipts = battle.ai_receipts();
    report.attempted_orders += receipts.len() as u32;
    report.accepted_orders += receipts.iter().filter(|receipt| receipt.accepted).count() as u32;
    report.refused_orders += receipts.iter().filter(|receipt| !receipt.accepted).count() as u32;
    for receipt in receipts.iter().filter(|receipt| !receipt.accepted) {
        *report
            .refused_errors
            .entry(format!("{:?}", receipt.error))
            .or_default() += 1;
        if let Some(player) = setup
            .players
            .iter()
            .find(|player| player.id == receipt.player)
        {
            report.refused_by_side[player.side.index()] += 1;
        }
    }
    report.battle_seconds = report.ticks as f64 / f64::from(inputs.rules.tick_hz);
    report.result = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .and_then(|view| view.result);
    report.air = battle
        .match_report()
        .map(|match_report| air_summary(&setup.rules, match_report));
    report.doctrine_pivots = battle.ai_pivots_by_side();
    if let Some(match_report) = battle.match_report() {
        let totals = match_report.side_totals(&setup.players);
        report.credit_spend = [totals[0].spent as u64, totals[1].spent as u64];
    }
    report.opening = [Vec::new(), Vec::new()];
    let preparation_ticks = u64::from(
        setup
            .skirmish
            .as_ref()
            .expect("tournament setup has skirmish rules")
            .rules
            .preparation_s,
    )
    .saturating_mul(u64::from(setup.rules.tick_hz));
    for (tick, command) in battle.replay().accepted {
        if tick > preparation_ticks {
            continue;
        }
        if let Order::ConfirmPurchase { variant, .. } = command.order {
            report.opening[command.side.index()].push(variant);
        }
    }
    report.replay_digest = contract::identity::json_hash(&battle.replay()).ok();
    let replay = battle.replay();
    report.replay_match =
        sim::battle::Battle::from_replay(&setup, &replay)
            .ok()
            .map(|mut replayed| {
                for _ in 0..report.ticks {
                    replayed.step();
                }
                replayed.digest() == report.battle_digest
            });
    report
}

/// Per side: what its jets did and what its ground AA did to the enemy's,
/// read from the match report's spawn and kill events.
fn air_summary(rules: &Rules, report: &contract::skirmish::MatchReport) -> [AirSide; 2] {
    use contract::catalog::AltitudeLayer;
    use contract::skirmish::MatchEvent;
    use sim::skirmish_ai::{is_jet, unit_engages_aircraft_in};
    let jet = |kind| is_jet(rules, kind);
    // A ground unit that can fight jets: by what its weapons engage up there.
    let ground_aa = |kind| {
        rules.catalog.get(kind).mobility.layer() == AltitudeLayer::Ground
            && unit_engages_aircraft_in(rules, kind, AltitudeLayer::HighAir)
    };
    let mut sides = [AirSide::default(), AirSide::default()];
    for event in &report.events {
        match event {
            MatchEvent::Spawn { tick, unit, cost } => {
                let side = &mut sides[unit.side.index()];
                if jet(unit.kind) {
                    side.jets_bought += 1;
                    side.jet_spend += cost;
                    side.first_jet_tick.get_or_insert(*tick);
                } else if ground_aa(unit.kind) {
                    side.aa_bought += 1;
                }
                side.spent += cost;
            }
            MatchEvent::Kill {
                tick,
                victim,
                victim_cost,
                killer,
                ..
            } => {
                sides[victim.side.index()].lost_value += victim_cost;
                let Some(killer) = killer.filter(|k| k.side != victim.side) else {
                    continue;
                };
                let by = &mut sides[killer.side.index()];
                let jet_victim = jet(victim.kind);
                if jet(killer.kind) {
                    by.jet_kills += 1;
                    by.jet_kill_value += victim_cost;
                    if jet_victim {
                        by.jet_air_kills += 1;
                    }
                }
                if jet_victim && ground_aa(killer.kind) {
                    by.aa_jet_kills += 1;
                    by.first_aa_jet_kill_tick.get_or_insert(*tick);
                }
                if jet_victim {
                    sides[victim.side.index()].jets_lost += 1;
                }
            }
            MatchEvent::Refund { unit, credits, .. } => {
                if jet(unit.kind) {
                    sides[unit.side.index()].jet_refunds += credits;
                }
            }
        }
    }
    sides
}

#[derive(Debug, Default, serde::Serialize)]
pub(crate) struct AirSide {
    spent: u32,
    lost_value: u32,
    jets_bought: u32,
    jet_spend: u32,
    /// Credits returned by jets that withdrew.
    jet_refunds: f64,
    first_jet_tick: Option<u64>,
    /// Enemy units this side's jets destroyed, and their price.
    jet_kills: u32,
    jet_kill_value: u32,
    /// Of those, enemy jets.
    jet_air_kills: u32,
    jets_lost: u32,
    aa_bought: u32,
    /// Enemy jets this side's ground AA destroyed.
    aa_jet_kills: u32,
    first_aa_jet_kill_tick: Option<u64>,
}
