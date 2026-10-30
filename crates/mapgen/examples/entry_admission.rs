//! Usage: cargo run -p mapgen --release --example entry_admission -- --seeds 30 [--types open,mixed,metro] [--pairings 1v1,2v2,3v3,5v5]

use contract::generation::{GenerationProfile, GenerationRequest, MapSize, MapType};
use std::collections::BTreeMap;
use std::io::Write;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const RULES: &str = include_str!("../../../fixtures/parity/map-layout/physical-rules.json");

#[derive(Clone, Copy)]
struct Pairing {
    name: &'static str,
    entries: usize,
    size: MapSize,
}

const PAIRINGS: [Pairing; 4] = [
    Pairing {
        name: "1v1",
        entries: 1,
        size: MapSize::Small,
    },
    Pairing {
        name: "2v2",
        entries: 2,
        size: MapSize::Medium,
    },
    Pairing {
        name: "3v3",
        entries: 3,
        size: MapSize::Large,
    },
    Pairing {
        name: "5v5",
        entries: 3,
        size: MapSize::Xl,
    },
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let seeds = value(&args, "--seeds").map_or(Ok(30), |value| value.parse::<u64>())?;
    let types = value(&args, "--types")
        .map(|value| {
            value
                .split(',')
                .map(parse_type)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_else(|| {
            vec![
                ("open", MapType::Open),
                ("mixed", MapType::Mixed),
                ("metro", MapType::Metro),
            ]
        });
    let pairings = value(&args, "--pairings")
        .map(|value| {
            value
                .split(',')
                .map(|name| {
                    PAIRINGS
                        .iter()
                        .copied()
                        .find(|pairing| pairing.name == name)
                        .ok_or_else(|| format!("unknown pairing {name}"))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_else(|| PAIRINGS.to_vec());

    let presets = mapgen::layout::PresetDefinitions::from_json(PRESETS)
        .map_err(|diagnostics| format!("invalid presets: {diagnostics:?}"))?;
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES)?)?;
    let rules: contract::scenario::Rules = serde_json::from_value(sim::fixtures::test_rules())?;
    let players: Vec<contract::players::Player> = serde_json::from_value(serde_json::json!([
        {"id":"blue-1","side":"blue","faction":"us","controller":"active_ai"},
        {"id":"red-1","side":"red","faction":"eastern","controller":"active_ai"}
    ]))?;
    let defaults: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/generated-battle.json"))?;
    let limits: contract::generation::CompileLimits =
        serde_json::from_value(defaults["limits"].clone())?;

    for (type_name, map_type) in types {
        for pairing in &pairings {
            let mut admitted = 0;
            let mut reasons = BTreeMap::<String, usize>::new();
            for seed in 1..=seeds {
                let request = GenerationRequest {
                    generator_version: mapgen::layout::GENERATOR_VERSION.into(),
                    preset_revision: presets.revision.clone(),
                    seed: seed.into(),
                    template_catalog_hash: catalogue.hash().into(),
                    map_type,
                    size: pairing.size,
                    entries_per_side: pairing.entries,
                    profile: GenerationProfile::Skirmish,
                    region: None,
                    limits,
                };
                match mapgen::generate_map(
                    &serde_json::to_string(&request)?,
                    PRESETS,
                    TEMPLATES,
                    RULES,
                ) {
                    mapgen::CompileOutcome::Ok { result } => {
                        let prepared = sim::encounter::PreparedMap::new(&result.map, &rules);
                        match sim::map_analysis::admit_skirmish(
                            &prepared.queries(&result.map, &result.sites),
                            &rules,
                            &players,
                        ) {
                            Ok(_) => admitted += 1,
                            Err(error) => {
                                *reasons.entry(admission_reason(&error).into()).or_default() += 1
                            }
                        }
                    }
                    mapgen::CompileOutcome::Error { diagnostics } => {
                        for diagnostic in diagnostics {
                            *reasons
                                .entry(generation_reason(&diagnostic.message).into())
                                .or_default() += 1;
                        }
                    }
                }
            }
            let reasons = reasons
                .into_iter()
                .map(|(check, count)| format!("{check}:{count}"))
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "{type_name} {} {} admitted={admitted}/{seeds} reasons={{{reasons}}}",
                pairing.name,
                pairing.size.name()
            );
            std::io::stdout().flush()?;
        }
    }
    Ok(())
}

fn value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn parse_type(name: &str) -> Result<(&'static str, MapType), String> {
    match name {
        "open" => Ok(("open", MapType::Open)),
        "mixed" => Ok(("mixed", MapType::Mixed)),
        "metro" => Ok(("metro", MapType::Metro)),
        _ => Err(format!("unknown map type {name}")),
    }
}

fn generation_reason(message: &str) -> &'static str {
    if message.contains("river") || message.contains("water") {
        "river_mouth"
    } else if message.contains("spacing") {
        "spacing"
    } else if message.contains("town") || message.contains("road") {
        "road_or_town"
    } else {
        "generation"
    }
}

fn admission_reason(message: &str) -> &'static str {
    if message.contains("cannot admit") {
        "entry_stands"
    } else if message.contains("no route") || message.contains("does not reach") {
        "entry_routes"
    } else if message.contains("travel") {
        "fairness"
    } else if message.contains("spacing") {
        "spacing"
    } else {
        "admission"
    }
}
