//! Deterministic two-controller skirmish corpus runner.
//!
//! This stays outside browser setup: each side observes its own frame, asks
//! the policy for ordinary orders, accepts them through Battle, and advances
//! one authoritative tick. The JSONL report is deliberately evidence rather
//! than a winner fixture.
use contract::catalog::Faction;
use contract::command::{CommandEnvelope, Order};
use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
use contract::generation::{GenerationProfile, GenerationRequest, MapSize, MapType};
use contract::ids::Side;
use contract::scenario::{Rules, ScenarioDefinition};
use contract::skirmish::{MatchRules, SkirmishSetup};
use contract::templates::TemplateGeometryCatalog;
use serde::{Deserialize, Serialize};
use sim::skirmish_ai::BalancedDoctrine;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    revision: u32,
    rows: Vec<Row>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    id: String,
    map: MapSource,
    seed: u64,
    horizon_ticks: u64,
    no_progress_ticks: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum MapSource {
    Saved(String),
    Generated(GeneratedMap),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedMap {
    kind: String,
    #[serde(rename = "type")]
    map_type: String,
    size: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    map_identity: Option<String>,
    site_count: u32,
    replay_digest: Option<String>,
    replay_match: Option<bool>,
    credit_spend: [u64; 2],
    living_cost: [u64; 2],
    recon_range_m: f64,
}

struct Inputs {
    presets: String,
    templates: String,
    rules: Rules,
    rules_json: String,
    preset_revision: String,
    template_hash: String,
    limits: contract::generation::CompileLimits,
}

impl Inputs {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let root = sim::fixtures::dir();
        let presets = std::fs::read_to_string(root.join("map-presets.json"))?;
        let preset_revision = serde_json::from_str::<serde_json::Value>(&presets)?["revision"]
            .as_str()
            .ok_or("map presets have no revision")?
            .to_owned();
        let templates = std::fs::read_to_string(root.join("prototype-building-templates.json"))?;
        let catalog = TemplateGeometryCatalog::new(serde_json::from_str(&templates)?)?;
        let rules: Rules = serde_json::from_value(sim::fixtures::game())?;
        let config: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
            root.join("generated-battle.json"),
        )?)?;
        Ok(Self {
            presets,
            templates,
            rules_json: serde_json::to_string(&rules)?,
            rules,
            preset_revision,
            template_hash: catalog.hash().into(),
            limits: serde_json::from_value(config["limits"].clone())?,
        })
    }
}

fn prepare_row(
    inputs: &Inputs,
    row: &Row,
) -> Result<
    (
        ScenarioDefinition,
        sim::encounter::PreparedMap,
        Option<String>,
    ),
    String,
> {
    let (map, prepared, sites, identity) = match &row.map {
        MapSource::Saved(id) => {
            let value = sim::fixtures::with_map(id);
            let map: contract::map::MapDefinition =
                serde_json::from_value(value["map"].clone()).map_err(|error| error.to_string())?;
            let prepared = sim::encounter::PreparedMap::new(&map, &inputs.rules);
            let sites = synthetic_sites(&map);
            let identity = contract::identity::json_hash(&map).map_err(|e| e.to_string())?;
            (map, prepared, sites, Some(identity))
        }
        MapSource::Generated(spec) => {
            if spec.kind != "generated" {
                return Err("generated map source must name kind=generated".into());
            }
            let map_type = match spec.map_type.as_str() {
                "open" => MapType::Open,
                "mixed" => MapType::Mixed,
                "metro" => MapType::Metro,
                other => return Err(format!("unknown generated map type {other}")),
            };
            let size = match spec.size.as_str() {
                "small" => MapSize::Small,
                "medium" => MapSize::Medium,
                "large" => MapSize::Large,
                "xl" => MapSize::Xl,
                other => return Err(format!("unknown generated map size {other}")),
            };
            let request = GenerationRequest {
                generator_version: mapgen::layout::GENERATOR_VERSION.into(),
                preset_revision: inputs.preset_revision.clone(),
                seed: row.seed.into(),
                template_catalog_hash: inputs.template_hash.clone(),
                map_type,
                size,
                profile: GenerationProfile::Skirmish,
                region: None,
                limits: inputs.limits,
            };
            let generated = mapgen::generate_map(
                &serde_json::to_string(&request).map_err(|e| e.to_string())?,
                &inputs.presets,
                &inputs.templates,
                &inputs.rules_json,
            );
            let mapgen::CompileOutcome::Ok { result } = generated else {
                return Err("map generation refused the pinned request".into());
            };
            let prepared = sim::encounter::PreparedMap::new(&result.map, &inputs.rules);
            let admitted = sim::map_analysis::admit_skirmish(
                &prepared.queries(&result.map, &result.sites),
                &inputs.rules,
            )
            .map_err(|error| format!("skirmish admission: {error}"))?;
            let sites = admitted
                .sites
                .skirmish
                .ok_or("admission returned no skirmish sites")?;
            let identity = serde_json::to_string(&result.identity).map_err(|e| e.to_string())?;
            (result.map, prepared, sites, Some(identity))
        }
    };
    let setup = ScenarioDefinition {
        skirmish: Some(SkirmishSetup {
            factions: [Faction::Us, Faction::Europe],
            ai_side: None,
            rules: MatchRules {
                credits_per_minute: 200,
                starting_minutes: 2,
                preparation_s: 20,
                max_units: 30,
                dispatch_interval_s: 1,
            },
            sites,
        }),
        map,
        rules: inputs.rules.clone(),
        units: vec![],
        events: vec![],
        scripts: vec![],
        opponent: None,
        encounter: None,
    };
    Ok((setup, prepared, identity))
}

fn synthetic_sites(map: &contract::map::MapDefinition) -> SkirmishSites {
    let [width, depth] = map.size;
    SkirmishSites {
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
    }
}

fn main() {
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../fixtures/skirmish-corpus.json"))
            .expect("valid skirmish corpus");
    assert_eq!(corpus.revision, 1, "unsupported corpus revision");
    let inputs = Inputs::load().expect("load generation inputs");
    for row in corpus.rows {
        println!(
            "{}",
            serde_json::to_string(&run_safe(&inputs, row)).unwrap()
        );
    }
}

fn run_safe(inputs: &Inputs, row: Row) -> Report {
    let fallback_id = row.id.clone();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(inputs, row))) {
        Ok(report) => report,
        Err(panic) => {
            let mut report = empty_report(fallback_id);
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

fn run(inputs: &Inputs, row: Row) -> Report {
    let mut report = empty_report(row.id.clone());
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
                let purchase_cost = match &order {
                    Order::ConfirmPurchase { variant, .. } => {
                        setup.rules.catalog.by_id(variant).cost as u64
                    }
                    _ => 0,
                };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_is_versioned_and_repeatable() {
        let corpus: Corpus =
            serde_json::from_str(include_str!("../../../fixtures/skirmish-corpus.json")).unwrap();
        assert_eq!(corpus.revision, 1);
        let inputs = Inputs::load().unwrap();
        let row = corpus.rows.into_iter().next().unwrap();
        let left = run(
            &inputs,
            Row {
                id: row.id.clone(),
                map: row.map.clone(),
                seed: row.seed,
                horizon_ticks: 90,
                no_progress_ticks: 30,
            },
        );
        let right = run(
            &inputs,
            Row {
                id: row.id,
                map: row.map,
                seed: row.seed,
                horizon_ticks: 90,
                no_progress_ticks: 30,
            },
        );
        assert_eq!(left.battle_digest, right.battle_digest);
        assert_eq!(left.replay_digest, right.replay_digest);
        assert_eq!(left.replay_match, Some(true));
        assert_eq!(left.site_count, 3);
        assert!(left.map_identity.is_some());
        assert_eq!(left.accepted_orders, right.accepted_orders);
        assert_eq!(left.refused_orders, right.refused_orders);
    }
}
