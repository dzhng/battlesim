use super::inputs::{prepare_row, Inputs};
use super::model::{ProgressSignature, Report, Row, Status};
use contract::catalog::Faction;
use contract::command::{CommandEnvelope, Order};
use contract::ids::Side;
use sim::skirmish_ai::BalancedDoctrine;

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
        accepted_orders: 0,
        attempted_orders: 0,
        policy: "balanced_doctrine",
        factions: [Faction::Us, Faction::Europe],
        diagnostic: None,
        result: None,
        objective_first_owner_ticks: std::collections::BTreeMap::new(),
        first_contact_tick: None,
        recon_sightings: 0,
        doctrine_pivots: [0, 0],
        map_identity: None,
        site_count: 0,
        replay_digest: None,
        replay_match: None,
        credit_spend: [0, 0],
        living_cost: [0, 0],
        recon_range_m: 0.0,
    }
}

pub(crate) fn run(inputs: &Inputs, row: Row) -> Report {
    let mut report = empty_report(row.id.clone());
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
    let mut stagnant = 0;
    let mut previous: Option<ProgressSignature> = None;
    while report.ticks < row.horizon_ticks {
        let mut refused_this_tick = 0;
        for side in Side::ALL {
            let frame = battle.observe(side).clone();
            let orders = policies[side.index()].decide(
                side,
                report.factions[side.index()],
                &setup.skirmish.as_ref().unwrap().sites.entries[side.index()],
                &frame,
                &setup.rules,
                &mut |id, goal| battle.route_time(side, id, goal),
            );
            for order in orders {
                let purchase_cost = match &order {
                    Order::ConfirmPurchase { variant, .. } => {
                        setup.rules.catalog.by_id(variant).cost as u64
                    }
                    _ => 0,
                };
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
                } else {
                    report.accepted_orders += 1;
                    report.credit_spend[side.index()] += purchase_cost;
                }
            }
        }
        report.refused_orders += refused_this_tick;
        battle.step();
        report.ticks += 1;
        for side in Side::ALL {
            let frame = battle.observe(side);
            if !frame.contacts.is_empty() || !frame.identified.is_empty() {
                report.first_contact_tick.get_or_insert(frame.tick);
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
        for side in Side::ALL {
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
