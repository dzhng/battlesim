//! Deterministic two-controller skirmish corpus runner.
//!
//! This stays outside browser setup: each side observes its own frame, asks
//! the policy for ordinary orders, accepts them through Battle, and advances
//! one authoritative tick. The JSONL report is deliberately evidence rather
//! than a winner fixture.
use contract::catalog::Faction;
use contract::command::CommandEnvelope;
use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
use contract::ids::Side;
use contract::scenario::{Rules, ScenarioDefinition};
use contract::skirmish::{MatchRules, SkirmishSetup};
use serde::{Deserialize, Serialize};
use sim::skirmish_ai::BalancedDoctrine;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    revision: u32,
    rows: Vec<Row>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    id: String,
    map: String,
    seed: u64,
    horizon_ticks: u64,
    no_progress_ticks: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Complete,
    Refused,
    Timeout,
    NoProgress,
}

#[derive(Debug, Serialize)]
struct Report {
    id: String,
    status: Status,
    ticks: u64,
    battle_digest: u64,
    refused_orders: u32,
    accepted_orders: u32,
    attempted_orders: u32,
    policy: &'static str,
    factions: [Faction; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<String>,
    result: Option<contract::skirmish::MatchResult>,
    objective_first_owner_ticks: std::collections::BTreeMap<String, u64>,
    first_contact_tick: Option<u64>,
    recon_sightings: u32,
    doctrine_pivots: [u32; 2],
}

fn main() {
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../fixtures/skirmish-corpus.json"))
            .expect("valid skirmish corpus");
    assert_eq!(corpus.revision, 1, "unsupported corpus revision");
    for row in corpus.rows {
        println!("{}", serde_json::to_string(&run(row)).unwrap());
    }
}

fn run(row: Row) -> Report {
    let mut report = Report {
        id: row.id.clone(),
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
    };
    let value = std::panic::catch_unwind(|| sim::fixtures::with_map(&row.map));
    let Ok(mut value) = value else { return report };
    let mut game = sim::fixtures::game();
    game["map"] = value["map"].take();
    game["units"] = serde_json::json!([]);
    game["events"] = serde_json::json!([]);
    game["scripts"] = serde_json::json!([]);
    let map = match serde_json::from_value(game["map"].clone()) {
        Ok(map) => map,
        Err(error) => {
            report.diagnostic = Some(error.to_string());
            return report;
        }
    };
    let rules = match serde_json::from_value::<Rules>(game) {
        Ok(rules) => rules,
        Err(error) => {
            report.diagnostic = Some(error.to_string());
            return report;
        }
    };
    let mut setup = ScenarioDefinition {
        skirmish: None,
        map,
        rules,
        units: vec![],
        events: vec![],
        scripts: vec![],
        opponent: None,
        encounter: None,
    };
    let [width, depth] = setup.map.size;
    setup.skirmish = Some(SkirmishSetup {
        factions: report.factions,
        ai_side: None,
        rules: MatchRules {
            credits_per_minute: 200,
            starting_minutes: 2,
            preparation_s: 20,
            max_units: 30,
            dispatch_interval_s: 1,
        },
        sites: SkirmishSites {
            entries: [
                EntrySite {
                    side: Side::Blue,
                    center: [width * 0.5, 12.0],
                    yaw: 1.57,
                },
                EntrySite {
                    side: Side::Red,
                    center: [width * 0.5, depth - 12.0],
                    yaw: -1.57,
                },
            ],
            objectives: vec![
                ObjectiveSite {
                    id: "center".into(),
                    center: [width * 0.5, depth * 0.5],
                    radius_m: 50.0,
                    kind: ObjectiveSiteKind::Junction,
                    counterpart: None,
                },
                ObjectiveSite {
                    id: "west".into(),
                    center: [width * 0.25, depth * 0.5],
                    radius_m: 50.0,
                    kind: ObjectiveSiteKind::Field,
                    counterpart: Some("east".into()),
                },
                ObjectiveSite {
                    id: "east".into(),
                    center: [width * 0.75, depth * 0.5],
                    radius_m: 50.0,
                    kind: ObjectiveSiteKind::Field,
                    counterpart: Some("west".into()),
                },
            ],
            candidates: vec![],
        },
    });
    let mut battle = sim::battle::Battle::new(&setup, row.seed);
    let mut policies = [
        BalancedDoctrine::new(contract::random::mix(row.seed ^ 0xB1)),
        BalancedDoctrine::new(contract::random::mix(row.seed ^ 0xB2)),
    ];
    let mut seq = [1u64, 1u64];
    let mut stagnant = 0;
    let mut previous = (0u32, 0u32, 0u64, 0u64, 0u64);
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
                report.attempted_orders += 1;
                let ack = battle.accept(CommandEnvelope {
                    side,
                    seq: seq[side.index()],
                    order,
                    queued: false,
                });
                seq[side.index()] += 1;
                if ack.error.is_some() {
                    refused_this_tick += 1;
                } else {
                    report.accepted_orders += 1;
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
                .filter(|unit| !unit.sees.is_empty())
                .map(|unit| unit.sees.len() as u32)
                .sum::<u32>();
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
                (
                    owned,
                    capture,
                    v.scores.iter().sum::<f64>().to_bits(),
                    capture_bits,
                    observed,
                )
            })
            .unwrap_or_default();
        stagnant = if progress == previous {
            stagnant + 1
        } else {
            0
        };
        previous = progress;
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
    report.result = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .and_then(|view| view.result);
    report.doctrine_pivots = [policies[0].pivot_count(), policies[1].pivot_count()];
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_is_versioned_and_repeatable() {
        let corpus: Corpus =
            serde_json::from_str(include_str!("../../../fixtures/skirmish-corpus.json")).unwrap();
        assert_eq!(corpus.revision, 1);
        let row = corpus.rows.into_iter().next().unwrap();
        let left = run(Row {
            id: row.id.clone(),
            map: row.map.clone(),
            seed: row.seed,
            horizon_ticks: 90,
            no_progress_ticks: 30,
        });
        let right = run(Row {
            id: row.id,
            map: row.map,
            seed: row.seed,
            horizon_ticks: 90,
            no_progress_ticks: 30,
        });
        assert_eq!(left.battle_digest, right.battle_digest);
        assert_eq!(left.accepted_orders, right.accepted_orders);
        assert_eq!(left.refused_orders, right.refused_orders);
    }
}
