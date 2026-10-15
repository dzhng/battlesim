use super::inputs::{prepare_row, Inputs};
use super::model::{ProgressSignature, Report, Row, Status};
use contract::catalog::Faction;
use contract::command::{CommandEnvelope, Order};
use contract::ids::Side;
use contract::observation::ObservationFrame;
use contract::scenario::Rules;
use sim::skirmish_ai::BalancedDoctrine;
use std::collections::BTreeMap;

pub(crate) fn run_safe(inputs: &Inputs, row: Row) -> Report {
    let fallback_id = row.id.clone();
    let seed = row.seed;
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(inputs, row))) {
        Ok(report) => report,
        Err(panic) => {
            let mut report = empty_report(fallback_id);
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

fn empty_report(id: String) -> Report {
    Report {
        id,
        seed: 0,
        max_battle_seconds: 0,
        battle_seconds: 0.0,
        status: Status::Refused,
        ticks: 0,
        battle_digest: 0,
        refused_orders: 0,
        refused_by_side: [0, 0],
        accepted_orders: 0,
        attempted_orders: 0,
        policy: "balanced_doctrine",
        factions: [Faction::Us, Faction::Europe],
        diagnostic: None,
        result: None,
        objective_first_owner_ticks: BTreeMap::new(),
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
    let mut report = empty_report(row.id.clone());
    report.factions = inputs.factions;
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
    let mut policies = [
        BalancedDoctrine::new(contract::random::mix(row.seed ^ 0xB1)),
        BalancedDoctrine::new(contract::random::mix(row.seed ^ 0xB2)),
    ];
    let mut seq = [1u64, 1u64];
    let mut next_buy = [0usize; 2];
    let mut stagnant = 0;
    let mut previous: Option<ProgressSignature> = None;
    while report.ticks < row.horizon_ticks {
        let mut refused_this_tick = 0;
        for side in Side::COMBATANTS {
            let frame = battle.observe(side).clone();
            let orders = policies[side.index()].decide(
                side,
                report.factions[side.index()],
                &setup.skirmish.as_ref().unwrap().sites.entries[side.index()],
                &frame,
                &setup.rules,
                &mut |id, goal| battle.route_time(side, id, goal),
            );
            let mut orders = orders;
            if let Some(list) = &inputs.buy[side.index()] {
                orders.retain(|order| !matches!(order, Order::ConfirmPurchase { .. }));
                orders.extend(scripted_purchases(
                    &setup.rules,
                    &frame,
                    list,
                    &mut next_buy[side.index()],
                ));
            }
            let preparing = frame
                .skirmish
                .as_ref()
                .is_some_and(|view| view.phase == contract::skirmish::Phase::Preparation);
            for order in orders {
                let purchase = match &order {
                    Order::ConfirmPurchase { variant, .. } => Some(variant.clone()),
                    _ => None,
                };
                let purchase_cost = purchase
                    .as_ref()
                    .map_or(0, |variant| setup.rules.catalog.by_id(variant).cost as u64);
                report.attempted_orders += 1;
                let ack = battle.accept(CommandEnvelope {
                    side,
                    slot: None,
                    seq: seq[side.index()],
                    order,
                    queued: false,
                });
                seq[side.index()] += 1;
                if ack.error.is_some() {
                    refused_this_tick += 1;
                    report.refused_by_side[side.index()] += 1;
                } else {
                    report.accepted_orders += 1;
                    report.credit_spend[side.index()] += purchase_cost;
                    if let Some(variant) = purchase.filter(|_| preparing) {
                        report.opening[side.index()].push(variant);
                    }
                }
            }
        }
        report.refused_orders += refused_this_tick;
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
                )
            })
            .unwrap_or_default();
        stagnant = if previous.as_ref().is_some_and(|old| *old == progress) {
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
    report.battle_seconds = report.ticks as f64 / f64::from(inputs.rules.tick_hz);
    report.result = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .and_then(|view| view.result);
    report.air = battle
        .match_report()
        .map(|match_report| air_summary(&setup.rules, match_report));
    report.doctrine_pivots = [policies[0].pivot_count(), policies[1].pivot_count()];
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

/// The next cards of `list` the wallet affords, strictly in order, so a side
/// saves for an expensive card rather than skipping it. Cards before a `|`
/// are bought once; the rest repeat. Each goes to the objective nearest the
/// middle of them all.
fn scripted_purchases(
    rules: &Rules,
    frame: &ObservationFrame,
    list: &[String],
    next: &mut usize,
) -> Vec<Order> {
    let Some(view) = frame.skirmish.as_ref() else {
        return vec![];
    };
    let n = view.objectives.len().max(1) as f64;
    let middle = view.objectives.iter().fold([0.0, 0.0], |m, o| {
        [m[0] + o.center[0] / n, m[1] + o.center[1] / n]
    });
    let destination = view
        .objectives
        .iter()
        .map(|o| o.center)
        .min_by(|a, b| {
            let d = |p: [f64; 2]| (p[0] - middle[0]).hypot(p[1] - middle[1]);
            d(*a).total_cmp(&d(*b))
        })
        .unwrap_or(middle);
    let bar = list.iter().position(|card| card == "|");
    let (mut credits, mut slots) = (view.credits, view.occupied_slots);
    let mut orders = Vec::new();
    while view.phase != contract::skirmish::Phase::Finished && slots < view.max_units {
        let index = match bar {
            Some(bar) if *next >= bar => bar + 1 + (*next - bar) % (list.len() - bar - 1).max(1),
            Some(_) => *next,
            None => *next % list.len(),
        };
        let Some(variant) = list.get(index) else {
            break;
        };
        let cost = f64::from(rules.catalog.by_id(variant).cost);
        if cost > credits {
            break;
        }
        credits -= cost;
        slots += 1;
        *next += 1;
        orders.push(Order::ConfirmPurchase {
            variant: variant.clone(),
            destination,
        });
    }
    orders
}

/// Per side: what its jets did and what its ground AA did to the enemy's,
/// read from the match report's spawn and kill events.
fn air_summary(rules: &Rules, report: &contract::skirmish::MatchReport) -> [AirSide; 2] {
    use contract::catalog::AltitudeLayer;
    use contract::skirmish::MatchEvent;
    let layer = |kind| rules.catalog.get(kind).mobility.layer();
    let ground_aa = |kind| {
        layer(kind) == AltitudeLayer::Ground
            && rules.catalog.mounts(kind).iter().any(|mount| {
                mount
                    .def
                    .weapons
                    .iter()
                    .filter_map(|weapon| rules.weapons.get(weapon))
                    .any(|weapon| weapon.targets.contains_key(&AltitudeLayer::HighAir))
            })
    };
    let mut sides = [AirSide::default(), AirSide::default()];
    for event in &report.events {
        match event {
            MatchEvent::Spawn { tick, unit, cost } => {
                let side = &mut sides[unit.side.index()];
                if layer(unit.kind) == AltitudeLayer::HighAir {
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
                let jet_victim = layer(victim.kind) == AltitudeLayer::HighAir;
                if layer(killer.kind) == AltitudeLayer::HighAir {
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
                if layer(unit.kind) == AltitudeLayer::HighAir {
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
