//! The sight circle on generated maps (M25): from every sample of open
//! ground, and from where the encounter planner stands each side's column,
//! the share of bearings on which an infantry eye sees to full range. One
//! JSON line for each map and arm, then a table. `bare` is the map before
//! the open-country pass, `furnished` the map the game makes.
//!
//! cargo run -p mapgen --release --example sight_report --
//!     [--seeds 3] [--seed <u64>] [--only mixed:small] [--step 200]
//!     [--arm bare|furnished] [--presets map-presets.json] [--out report.jsonl]
use contract::encounter::{EncounterRecipe, EncounterRecipes};
use contract::generation::{GenerationRequest, MapSize, MapType};
use contract::scenario::Rules;
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::PresetDefinitions;
use serde_json::{json, Value};
use sim::encounter::{plan_encounter, PreparedMap};
use sim::math::v2;
use std::io::Write;

use sim::map_analysis as sight;

const USAGE: &str = "usage: sight_report [--seeds <n>] [--seed <u64>] [--only <type>:<size>] \
[--step <metres>] [--arm bare|furnished] [--presets <map-presets.json>] [--out <report.jsonl>]";

struct Inputs {
    presets: PresetDefinitions,
    presets_json: String,
    templates_json: String,
    rules_json: String,
    catalogue: TemplateGeometryCatalog,
    rules: Rules,
    recipe: EncounterRecipe,
    encounter_seed: contract::identity::Seed,
    limits: mapgen::CompileLimits,
}

fn load(presets: Option<&str>) -> Result<Inputs, Box<dyn std::error::Error>> {
    let fixtures = sim::fixtures::dir();
    let read = |file| std::fs::read_to_string(fixtures.join(file));
    // Another presets file tries a tuning without editing the shipped one.
    let presets = match presets {
        Some(path) => std::fs::read_to_string(path)?,
        None => read("map-presets.json")?,
    };
    let presets_json = presets;
    let presets =
        PresetDefinitions::from_json(&presets_json).map_err(|errors| format!("{errors:?}"))?;
    let templates_json = read("prototype-building-templates.json")?;
    let catalogue = TemplateGeometryCatalog::new(serde_json::from_str(&templates_json)?)?;
    let rules_json = sim::fixtures::game().to_string();
    let config: Value = serde_json::from_str(&read("generated-battle.json")?)?;
    let recipes = EncounterRecipes::from_json(&read("encounters.json")?)?;
    Ok(Inputs {
        presets,
        catalogue,
        rules: serde_json::from_str(&rules_json)?,
        presets_json,
        templates_json,
        rules_json,
        recipe: recipes
            .recipes
            .get("assault")
            .ok_or("no assault recipe")?
            .clone(),
        encounter_seed: serde_json::from_value(config["encounter"]["seed"].clone())?,
        limits: serde_json::from_value(config["limits"].clone())?,
    })
}

/// One map's sight circles, in one arm.
fn run(inputs: &Inputs, request: &GenerationRequest, furnished: bool, step: f64) -> Value {
    let plan = if furnished {
        match mapgen::generate_plan(
            &serde_json::to_string(request).unwrap(),
            &inputs.presets_json,
            &inputs.templates_json,
            &inputs.rules_json,
        ) {
            mapgen::GenerateOutcome::Ok { plan } => Ok(*plan),
            mapgen::GenerateOutcome::Error { diagnostics } => Err(diagnostics),
        }
    } else {
        mapgen::layout::generate_layout(request, &inputs.presets).and_then(|layout| {
            mapgen::parcels::fill_districts(layout, request, &inputs.catalogue, &inputs.presets)
        })
    };
    let compiled = plan.and_then(|plan| {
        let country = mapgen::open_country::measure(&plan, &inputs.presets);
        let layout = mapgen::layout::measure(&plan, &inputs.presets);
        mapgen::lower(
            &mapgen::CompileRequest::generated(request, plan),
            &inputs.catalogue,
        )
        .map(|result| (result, country, layout))
    });
    let (result, country, layout) = match compiled {
        Ok(compiled) => compiled,
        Err(diagnostics) => return json!({"status": "refused", "diagnostics": diagnostics}),
    };
    let prepared = PreparedMap::new(&result.map, &inputs.rules);
    let world = &prepared.world;
    let eye = sight::infantry_sight(&inputs.rules);
    let bearings = sight::bearings(result.map.fog_cell_m, &eye);
    let share = |p| sight::open_share(world, &inputs.rules, &eye, p, bearings);
    let sampled: Vec<(sim::math::V2, f64)> = sight::sample_points(world, &result.sites, step)
        .into_iter()
        .filter_map(|p| share(p).map(|open| (p, open)))
        .collect();
    // Where the rule fails, to go and look.
    let unbroken: Vec<[f64; 2]> = sampled
        .iter()
        .filter(|(_, open)| *open >= 1.0)
        .take(12)
        .map(|(p, _)| [p.x, p.y])
        .collect();
    let mut shares: Vec<f64> = sampled.iter().map(|(_, open)| *open).collect();
    shares.sort_by(f64::total_cmp);
    let count = |test: &dyn Fn(f64) -> bool| shares.iter().filter(|s| test(**s)).count();
    // Where the planner stands each side's column.
    let planned = plan_encounter(
        &prepared.queries(&result.map, &result.sites),
        &inputs.rules,
        &inputs.recipe,
        inputs.encounter_seed,
    );
    let columns = match &planned {
        Err(diagnostics) => json!({"status": "refused", "diagnostics": diagnostics}),
        Ok(encounter) => {
            let sides: Vec<Value> = encounter
                .placement
                .deployments
                .iter()
                .map(|deployment| {
                    let mut open: Vec<f64> = deployment
                        .units
                        .iter()
                        .filter_map(|unit| {
                            let at = encounter.setup.units[*unit as usize].position;
                            share(v2(at[0], at[1]))
                        })
                        .collect();
                    open.sort_by(f64::total_cmp);
                    json!({
                        "side": deployment.side,
                        "head": deployment.head,
                        "units": open.len(),
                        "full_circles": open.iter().filter(|s| **s >= 1.0).count(),
                        "least_open": open.first(),
                        "most_open": open.last(),
                    })
                })
                .collect();
            json!({"status": "ok", "sides": sides})
        }
    };
    // Forests the one forest rule stood no tree on: ground that cuts no sight.
    let trunks: Vec<[f64; 2]> = world
        .props()
        .filter(|prop| prop.forest_tree)
        .map(|prop| [prop.center.x, prop.center.y])
        .collect();
    let treeless: Vec<[f64; 4]> = result
        .map
        .forests
        .iter()
        .map(|forest| (forest, forest.shape.limits()))
        .filter(|(forest, [x0, y0, x1, y1])| {
            !trunks.iter().any(|p| {
                p[0] >= *x0
                    && p[0] <= *x1
                    && p[1] >= *y0
                    && p[1] <= *y1
                    && forest.shape.contains(*p, 0.0)
            })
        })
        .map(|(_, limits)| limits)
        .collect();
    json!({
        "status": "ok",
        "treeless_forests": treeless,
        "sight_m": eye.range,
        "bearings": bearings,
        "points": shares.len(),
        "full_circles": count(&|s| s >= 1.0),
        "unbroken_at": unbroken,
        "enclosed": count(&|s| s < 0.5),
        "open": {
            "min": sight::quantile(&shares, 0.0),
            "p10": sight::quantile(&shares, 0.1),
            "p25": sight::quantile(&shares, 0.25),
            "median": sight::quantile(&shares, 0.5),
            "p75": sight::quantile(&shares, 0.75),
            "p90": sight::quantile(&shares, 0.9),
            "max": sight::quantile(&shares, 1.0),
        },
        "columns": columns,
        "country": country,
        "plain_share": layout.plain_share,
        "buildings": result.map.buildings.len(),
        "forests": result.map.forests.len(),
        "props": result.map.props.len(),
        "trees": world.props().filter(|prop| prop.forest_tree).count(),
        "authored_parts": result.report.authored_parts,
        "ground_points": result.report.ground_points,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut seeds: Vec<u64> = (1..=3).collect();
    let mut only: Option<(MapType, MapSize)> = None;
    let mut step = 200.0;
    let mut arms = vec![false, true];
    let mut out: Option<std::fs::File> = None;
    let mut presets: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(USAGE);
        match flag.as_str() {
            "--seeds" => seeds = (1..=value()?.parse()?).collect(),
            "--seed" => seeds = vec![value()?.parse()?],
            "--step" => step = value()?.parse()?,
            "--arm" => arms = vec![value()? == "furnished"],
            "--out" => out = Some(std::fs::File::create(value()?)?),
            "--presets" => presets = Some(value()?),
            "--only" => {
                let cell = value()?;
                let (map_type, size) = cell.split_once(':').ok_or(USAGE)?;
                only = Some((
                    serde_json::from_value(json!(map_type))?,
                    serde_json::from_value(json!(size))?,
                ));
            }
            _ => return Err(USAGE.into()),
        }
    }
    let inputs = load(presets.as_deref())?;
    println!(
        "{:<14} {:>20} {:<9} {:>6} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>6} {:>12} {:>5} {:>6} {:>6} {:>5} {:>5}",
        "map", "seed", "arm", "points", "full", "min", "p10", "p25", "med", "p90", "<50%",
        "columns", "homes", "line m", "copses", "trees", "cover"
    );
    for map_type in MapType::ALL {
        for size in MapSize::ALL {
            if only.is_some_and(|cell| cell != (map_type, size)) {
                continue;
            }
            for seed in &seeds {
                let request = GenerationRequest {
                    generator_version: mapgen::layout::GENERATOR_VERSION.into(),
                    preset_revision: inputs.presets.revision.clone(),
                    seed: (*seed).into(),
                    template_catalog_hash: inputs.catalogue.hash().into(),
                    map_type,
                    size,
                    profile: contract::generation::GenerationProfile::Standard,
                    region: None,
                    limits: inputs.limits,
                };
                for furnished in &arms {
                    let arm = if *furnished { "furnished" } else { "bare" };
                    let mut row = run(&inputs, &request, *furnished, step);
                    row["map"] = json!(format!("{}:{}", map_type.name(), size.name()));
                    row["seed"] = json!(seed.to_string());
                    row["arm"] = json!(arm);
                    if let Some(file) = &mut out {
                        writeln!(file, "{row}")?;
                    }
                    let name = format!("{}:{}", map_type.name(), size.name());
                    if row["status"] != "ok" {
                        println!(
                            "{name:<14} {seed:>20} {arm:<9} refused: {}",
                            row["diagnostics"]
                        );
                        continue;
                    }
                    let open = |key: &str| row["open"][key].as_f64().unwrap_or(f64::NAN) * 100.0;
                    let columns = match row["columns"]["sides"].as_array() {
                        None => "refused".to_string(),
                        Some(sides) => sides
                            .iter()
                            .map(|side| {
                                format!(
                                    "{:.0}",
                                    side["most_open"].as_f64().unwrap_or(f64::NAN) * 100.0
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("/"),
                    };
                    let sum = |key: &str| {
                        row["country"][key]["top"].as_f64().unwrap_or(0.0)
                            + row["country"][key]["bottom"].as_f64().unwrap_or(0.0)
                    };
                    println!(
                        "{name:<14} {seed:>20} {arm:<9} {:>6} {:>5} {:>5.0} {:>5.0} {:>5.0} {:>5.0} {:>5.0} {:>5.1}% {:>12} {:>5.0} {:>6.0} {:>6.0} {:>5.0} {:>5.0}",
                        row["points"],
                        row["full_circles"],
                        open("min"),
                        open("p10"),
                        open("p25"),
                        open("median"),
                        open("p90"),
                        row["enclosed"].as_f64().unwrap_or(0.0)
                            / row["points"].as_f64().unwrap_or(1.0).max(1.0)
                            * 100.0,
                        columns,
                        sum("homes"),
                        sum("tree_line_m"),
                        sum("copses"),
                        sum("trees"),
                        sum("cover"),
                    );
                }
            }
        }
    }
    Ok(())
}
