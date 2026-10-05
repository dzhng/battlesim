//! Integrated generation → assault placement → short battle, one JSON line
//! per exact request. Refusals remain rows; no failed seed is replaced.
//!
//! cargo run -p mapgen --release --example battle_sweep --
//!     [--seeds 10] [--seconds 30] [--only metro:small] [--region paris]
//!     [--out report.jsonl]
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::encounter::{EncounterRecipe, EncounterRecipes};
use contract::generation::{GenerationRequest, MapSize, MapType};
use contract::ids::UnitId;
use contract::scenario::{Rules, ScenarioDefinition};
use contract::templates::TemplateGeometryCatalog;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::encounter::{plan_encounter, PreparedMap};
use sim::math::{v2, V2};
use std::collections::BTreeMap;
use std::io::Write;
use std::time::Instant;

#[path = "../../sim/examples/common/instructions.rs"]
mod instructions;

struct Inputs {
    presets: String,
    templates: String,
    /// The explicit rule record whose physical fields generation reads.
    rules_json: String,
    rules: Rules,
    recipe: EncounterRecipe,
    request: GenerationRequest,
    identity: Value,
    encounter_seed: contract::identity::Seed,
}

impl Inputs {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let fixtures = sim::fixtures::dir();
        let read = |file| std::fs::read_to_string(fixtures.join(file));
        let presets = read("map-presets.json")?;
        let revision = serde_json::from_str::<Value>(&presets)?["revision"]
            .as_str()
            .ok_or("presets have no revision")?
            .to_owned();
        let templates = read("prototype-building-templates.json")?;
        let catalogue = TemplateGeometryCatalog::new(serde_json::from_str(&templates)?)?;
        let config: Value = serde_json::from_str(&read("generated-battle.json")?)?;
        let recipes = EncounterRecipes::from_json(&read("encounters.json")?)?;
        let rules: Rules = serde_json::from_value(sim::fixtures::game())?;
        let recipe = recipes
            .recipes
            .get("assault")
            .ok_or("no assault recipe")?
            .clone();
        let encounter_seed = serde_json::from_value(config["encounter"]["seed"].clone())?;
        let identity = json!({
            "presets_hash": contract::identity::json_hash(&serde_json::from_str::<Value>(&presets)?)?,
            "templates_hash": catalogue.hash(),
            "rules_hash": contract::identity::json_hash(&rules)?,
            "recipe_hash": contract::identity::json_hash(&recipe)?,
            "encounter_seed": encounter_seed,
            "battle_seed": 1,
            "engine_build": sim::battle::ENGINE_BUILD_ID,
        });
        Ok(Self {
            presets,
            templates,
            rules_json: serde_json::to_string(&rules)?,
            rules,
            recipe,
            identity,
            encounter_seed,
            request: GenerationRequest {
                generator_version: mapgen::layout::GENERATOR_VERSION.into(),
                preset_revision: revision,
                seed: 1.into(),
                template_catalog_hash: catalogue.hash().into(),
                map_type: MapType::Open,
                size: MapSize::Small,
                region: None,
                limits: serde_json::from_value(config["limits"].clone())?,
            },
        })
    }
}

struct Options {
    seeds: u64,
    seconds: u64,
    only: Option<(MapType, MapSize)>,
}

struct Progress {
    goal: Option<V2>,
    placed: Option<bool>,
    start: V2,
    last: V2,
    travelled_m: f64,
    near_goal_s: Option<f64>,
    first_moved_s: Option<f64>,
    state_ticks: BTreeMap<String, u64>,
}

fn play(setup: &ScenarioDefinition, command: CommandEnvelope, seed: u64, seconds: u64) -> Value {
    let (mut battle, built) = measured(|| Battle::new(setup, seed));
    let mut progress: Vec<_> = setup
        .units
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let at = battle.unit(UnitId(i as u32)).unwrap().position.xy();
            Progress {
                goal: None,
                placed: None,
                start: at,
                last: at,
                travelled_m: 0.0,
                near_goal_s: None,
                first_moved_s: None,
                state_ticks: BTreeMap::new(),
            }
        })
        .collect();
    let ack = battle.accept(command);
    if let Some(placement) = &ack.placement {
        for destination in &placement.destinations {
            let p = &mut progress[destination.unit.0 as usize];
            p.goal = Some(v2(destination.goal[0], destination.goal[1]));
            p.placed = Some(destination.placed);
        }
    }
    let refused = ack.error.is_some()
        || ack
            .placement
            .as_ref()
            .is_some_and(|placement| placement.destinations.iter().any(|d| !d.placed));
    let hz = setup.rules.tick_hz as u64;
    let ticks = seconds.checked_mul(hz).expect("duration fits a tick count");
    let mut times = Vec::new();
    let mut total = instructions::instructions().map(|_| 0u64);
    let mut peak = total;
    let mut peak_tick = None;
    let mut planning = (0u64, 0u64);
    for t in 1..=ticks {
        let before = instructions::instructions();
        let started = Instant::now();
        battle.step();
        let elapsed = started.elapsed();
        if let Some(cost) = instructions::instructions()
            .zip(before)
            .map(|(a, b)| a.saturating_sub(b))
        {
            *total.as_mut().unwrap() += cost;
            if cost > peak.unwrap() {
                peak = Some(cost);
                peak_tick = Some(t);
            }
        }
        times.push(elapsed.as_secs_f64() * 1000.0);
        let work = battle.load().planning_work;
        planning.0 += work;
        planning.1 = planning.1.max(work);
        let now = t as f64 / hz as f64;
        for (i, p) in progress.iter_mut().enumerate() {
            let unit = battle.unit(UnitId(i as u32)).unwrap();
            // Defender scripts have their own offset destinations. Keep the
            // first actual movement goal, even if later combat replaces it.
            if p.goal.is_none() {
                if let Some(order) = unit.orders.front().and_then(|o| o.movement()) {
                    p.goal = Some(order.destination);
                }
            }
            let at = unit.position.xy();
            p.travelled_m += (at - p.last).length();
            p.last = at;
            if p.first_moved_s.is_none() && (at - p.start).length() > 0.01 {
                p.first_moved_s = Some(now);
            }
            // Proximity is reported separately from the movement state; a
            // short window does not prove a long route completes or is stuck.
            if unit.alive()
                && p.placed != Some(false)
                && p.near_goal_s.is_none()
                && p.goal.is_some_and(|goal| (at - goal).length() < 10.0)
            {
                p.near_goal_s = Some(now);
            }
            let state = if unit.alive() {
                serde_json::to_value(unit.state).unwrap()
            } else {
                json!("dead")
            };
            *p.state_ticks
                .entry(state.as_str().unwrap().into())
                .or_default() += 1;
        }
    }
    let goals_not_neared: Vec<_> = progress
        .iter()
        .enumerate()
        .filter(|(_, p)| p.goal.is_some() && p.near_goal_s.is_none())
        .map(|(i, _)| UnitId(i as u32))
        .collect();
    let units: Vec<_> = progress
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let unit = battle.unit(UnitId(i as u32)).unwrap();
            let xy = |p: V2| [p.x, p.y];
            json!({"unit": unit.id, "kind": setup.units[i].kind, "side": unit.side,
            "goal": p.goal.map(xy), "placed": p.placed,
            "start": xy(p.start), "end": xy(p.last), "living": unit.alive(),
            "remaining_m": p.goal.map(|goal| (p.last-goal).length()),
            "travelled_m": p.travelled_m, "near_goal_s": p.near_goal_s,
            "first_moved_s": p.first_moved_s, "state": unit.state,
            "state_ticks": p.state_ticks})
        })
        .collect();
    let slow_ticks = times.iter().filter(|&&t| t > 33.0).count();
    times.sort_by(f64::total_cmp);
    let quantile = |q: f64| {
        times
            .get(((times.len() - 1) as f64 * q).round() as usize)
            .copied()
    };
    json!({"status": if refused {"command_refused"} else {"complete"},
    "command_ack": ack, "ticks": ticks, "seconds": seconds, "goal_near_m": 10,
    "units": units, "goals_not_neared": goals_not_neared, "digest": format!("{:016x}", battle.digest()),
    "rounds_launched": battle.load().rounds_launched,
    "cost": {"battle_build": built, "ticks": {
        "instructions": total, "mean_instructions": total.map(|n| n / ticks),
        "max_instructions": peak, "max_instruction_tick": peak_tick,
        "wall_ms": {"p50": quantile(0.5), "p95": quantile(0.95), "p99": quantile(0.99), "max": times.last()},
        "ticks_over_33_ms": slow_ticks, "planning_work": planning.0,
        "max_planning_work": planning.1,
    }}})
}

fn measured<T>(run: impl FnOnce() -> T) -> (T, Value) {
    let before = instructions::instructions();
    let started = Instant::now();
    let result = run();
    let wall_ms = started.elapsed().as_secs_f64() * 1000.0;
    let retired = instructions::instructions()
        .zip(before)
        .map(|(a, b)| a.saturating_sub(b));
    (result, json!({"instructions": retired, "wall_ms": wall_ms}))
}

fn run_case(inputs: &Inputs, request: &GenerationRequest, seconds: u64) -> Value {
    let (generated, cost) = measured(|| {
        mapgen::generate_map(
            &serde_json::to_string(request).unwrap(),
            &inputs.presets,
            &inputs.templates,
            &inputs.rules_json,
        )
    });
    match generated {
        mapgen::CompileOutcome::Error { diagnostics } => json!({
            "status": "refused", "stage": "generation", "diagnostics": diagnostics,
            "cost": {"generation": cost},
        }),
        mapgen::CompileOutcome::Ok { result } => {
            let (prepared, prepare_cost) =
                measured(|| PreparedMap::new(&result.map, &inputs.rules));
            let (planned, plan_cost) = measured(|| {
                plan_encounter(
                    &prepared.queries(&result.map, &result.sites),
                    &inputs.rules,
                    &inputs.recipe,
                    inputs.encounter_seed,
                )
            });
            drop(prepared);
            let mut row = match planned {
                Err(diagnostics) => {
                    json!({"status": "refused", "stage": "encounter", "diagnostics": diagnostics})
                }
                Ok(encounter) => {
                    let units = encounter
                        .placement
                        .deployments
                        .iter()
                        .find(|d| d.side == inputs.recipe.attacker)
                        .unwrap()
                        .units
                        .iter()
                        .map(|&id| UnitId(id))
                        .collect();
                    let command = CommandEnvelope {
                        side: inputs.recipe.attacker,
                        seq: 1,
                        queued: false,
                        order: Order::Move {
                            units,
                            gesture: 1,
                            goal: encounter.placement.objective.center,
                            route: RoutePolicy::Fastest,
                            direction: MoveDirection::Forward,
                            facing: None,
                        },
                    };
                    let placement = encounter.placement;
                    let setup = encounter.setup.scenario(result.map, inputs.rules.clone());
                    let mut row = play(&setup, command, 1, seconds);
                    row["placement"] = serde_json::to_value(placement).unwrap();
                    row
                }
            };
            row["identity"] = serde_json::to_value(result.identity).unwrap();
            if row["cost"].is_null() {
                row["cost"] = json!({});
            }
            row["cost"]["generation"] = cost;
            row["cost"]["preparation"] = prepare_cost;
            row["cost"]["planning"] = plan_cost;
            row
        }
    }
}

fn sweep(inputs: &Inputs, options: &Options, out: &mut dyn Write) -> std::io::Result<bool> {
    let mut passed = true;
    for map_type in MapType::ALL {
        for size in MapSize::ALL {
            if options.only.is_some_and(|cell| cell != (map_type, size)) {
                continue;
            }
            for seed in 1..=options.seeds {
                let mut request = inputs.request.clone();
                request.map_type = map_type;
                request.size = size;
                request.seed = seed.into();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_case(inputs, &request, options.seconds)
                }));
                let mut row = result.unwrap_or_else(|panic| json!({
                    "status": "panic",
                    "message": panic.downcast_ref::<String>().map(String::as_str)
                        .or_else(|| panic.downcast_ref::<&str>().copied()).unwrap_or("non-text panic"),
                }));
                passed &= row["status"] == "complete";
                row["request"] = serde_json::to_value(request).unwrap();
                row["inputs"] = inputs.identity.clone();
                serde_json::to_writer(&mut *out, &row)?;
                writeln!(out)?;
                out.flush()?;
            }
        }
    }
    Ok(passed)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let usage =
        "battle_sweep [--seeds 10] [--seconds 30] [--only metro:small] [--region paris] [--out report.jsonl]";
    let mut options = Options {
        seeds: 10,
        seconds: 30,
        only: None,
    };
    let mut output = None;
    let mut region = None;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--help" {
            println!("{usage}");
            return Ok(());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("{flag} takes a value; {usage}"))?;
        match flag.as_str() {
            "--seeds" => options.seeds = value.parse()?,
            "--seconds" => options.seconds = value.parse()?,
            "--out" => output = Some(value),
            "--region" => region = Some(value),
            "--only" => {
                options.only = MapType::ALL
                    .into_iter()
                    .flat_map(|map_type| MapSize::ALL.into_iter().map(move |size| (map_type, size)))
                    .find(|(t, s)| value == format!("{}:{}", t.name(), s.name()));
                if options.only.is_none() {
                    return Err(format!("unknown cell {value}; {usage}").into());
                }
            }
            _ => return Err(format!("unknown flag {flag}; {usage}").into()),
        }
    }
    if options.seeds == 0 || options.seconds == 0 {
        return Err("seeds and seconds must be positive".into());
    }
    let mut inputs = Inputs::load()?;
    inputs.request.region = region;
    let mut out: Box<dyn Write> = match output {
        Some(path) => Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::BufWriter::new(std::io::stdout().lock())),
    };
    if !sweep(&inputs, &options, &mut out)? {
        return Err("one or more requests failed; all outcomes are in the report".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract::ids::Side;

    #[test]
    fn every_refused_request_keeps_its_seed_cell_and_reason() {
        let mut inputs = Inputs::load().unwrap();
        inputs.request.generator_version = "obsolete-generator".into();
        let mut output = Vec::new();
        let passed = sweep(
            &inputs,
            &Options {
                seeds: 2,
                seconds: 1,
                only: None,
            },
            &mut output,
        )
        .unwrap();
        assert!(!passed, "a refused matrix cannot be reported as passed");
        let text = String::from_utf8(output).unwrap();
        let rows: Vec<Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            rows.len(),
            MapType::ALL.len() * MapSize::ALL.len() * 2,
            "every exact request needs an outcome, including refusals"
        );
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(
                row["request"]["type"],
                MapType::ALL[index / (MapSize::ALL.len() * 2)].name()
            );
            assert_eq!(
                row["request"]["size"],
                MapSize::ALL[(index / 2) % MapSize::ALL.len()].name()
            );
            assert_eq!(row["request"]["seed"], (index % 2 + 1).to_string());
            assert_eq!(row["status"], "refused");
            assert_eq!(row["stage"], "generation");
            assert_eq!(row["diagnostics"][0]["code"], "invalid_request");
            assert!(row["diagnostics"][0]["message"]
                .as_str()
                .unwrap()
                .contains("obsolete-generator"));
        }
    }

    #[test]
    fn short_battle_reports_each_acknowledged_goal_and_keeps_nonarrivals() {
        let rules: Rules = serde_json::from_value(sim::fixtures::game()).unwrap();
        let casualties = rules.catalog.by_id("rifle").squad_size();
        let setup: ScenarioDefinition = serde_json::from_value(json!({
            "map": {"size": [512,512], "fog_cell_m": 8, "height_grid_m": 4,
                "slope_cutoff_deg": 35},
            "rules": rules,
            "units": [
                {"side": "blue", "kind": "jeep", "position": [20,40]},
                {"side": "blue", "kind": "jeep", "position": [20,70]},
                {"side": "red", "kind": "rifle", "position": [450,450], "condition": {"casualties": casualties}}
            ]
        }))
        .unwrap();
        let command = CommandEnvelope {
            side: Side::Blue,
            seq: 1,
            queued: false,
            order: Order::Move {
                units: vec![UnitId(0), UnitId(1)],
                gesture: 1,
                goal: [200.0, 60.0],
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Forward,
                facing: None,
            },
        };
        let expected = Battle::new(&setup, 1)
            .accept(command.clone())
            .placement
            .unwrap();
        assert_ne!(expected.destinations[0].goal, expected.destinations[1].goal);
        let report = play(&setup, command, 1, 1);
        let rows = report["units"].as_array().unwrap();
        assert_eq!(rows.len(), 3, "uncommanded units also remain in the report");
        for (i, destination) in expected.destinations.iter().enumerate() {
            assert_eq!(rows[i]["unit"], json!(destination.unit));
            assert_eq!(rows[i]["goal"], json!(destination.goal));
            assert_eq!(rows[i]["placed"], destination.placed);
            assert!(rows[i]["travelled_m"].as_f64().unwrap() > 0.0);
            assert!(rows[i]["remaining_m"].as_f64().unwrap() > 100.0);
            assert!(rows[i]["near_goal_s"].is_null());
        }
        assert_eq!(rows[2]["unit"], 2);
        assert!(rows[2]["goal"].is_null());
        assert_eq!(rows[2]["living"], false);
        assert_eq!(rows[2]["state_ticks"]["dead"], setup.rules.tick_hz);
        assert!(rows[2]["state_ticks"]["idle"].is_null());
        assert_eq!(report["goals_not_neared"], json!([0, 1]));
        assert_eq!(report["ticks"], setup.rules.tick_hz);
    }
}
