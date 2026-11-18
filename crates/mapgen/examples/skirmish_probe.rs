//! Bounded complete-generation and native-route probe.
//! cargo run -p mapgen --example skirmish_probe -- [type] [size] [seeds] [first-seed] [players-per-side]
use contract::generation::{GenerationProfile, GenerationRequest, MapSize, MapType};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let map_type = match args.first().map(String::as_str).unwrap_or("open") {
        "open" => MapType::Open,
        "mixed" => MapType::Mixed,
        "metro" => MapType::Metro,
        _ => panic!("type: open/mixed/metro"),
    };
    let size = match args.get(1).map(String::as_str).unwrap_or("small") {
        "small" => MapSize::Small,
        "medium" => MapSize::Medium,
        "large" => MapSize::Large,
        "xl" => MapSize::Xl,
        _ => panic!("size: small/medium/large/xl"),
    };
    let seeds = args
        .get(2)
        .map(|s| s.parse::<u64>().unwrap())
        .unwrap_or(3)
        .min(200);
    let presets = include_str!("../../../fixtures/map-presets.json");
    let templates = include_str!("../../../fixtures/prototype-building-templates.json");
    let catalog =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(templates).unwrap())
            .unwrap();
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let per_side = args.get(4).map(|s| s.parse::<u8>().unwrap()).unwrap_or(1);
    let players: Vec<contract::players::Player> = serde_json::from_value(serde_json::Value::Array(
        (1..=per_side)
            .flat_map(|n| {
                [
                    serde_json::json!({"id":format!("blue-{n}"),"side":"blue","faction":"us","controller":"active_ai"}),
                    serde_json::json!({"id":format!("red-{n}"),"side":"red","faction":"eastern","controller":"active_ai"}),
                ]
            })
            .collect(),
    ))
    .unwrap();
    let rules_json = serde_json::to_string(&rules).unwrap();
    let presets_def = mapgen::layout::PresetDefinitions::from_json(presets).unwrap();
    let defaults: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/generated-battle.json")).unwrap();
    let first = args.get(3).map(|s| s.parse::<u64>().unwrap()).unwrap_or(1);
    for seed in first..first + seeds {
        let request = GenerationRequest {
            generator_version: mapgen::layout::GENERATOR_VERSION.into(),
            preset_revision: presets_def.revision.clone(),
            seed: seed.into(),
            template_catalog_hash: catalog.hash().into(),
            map_type,
            size,
            entries_per_side: per_side.into(),
            profile: GenerationProfile::Skirmish,
            region: None,
            limits: serde_json::from_value(defaults["limits"].clone()).unwrap(),
        };
        let started = std::time::Instant::now();
        let result = match mapgen::generate_map(
            &serde_json::to_string(&request).unwrap(),
            presets,
            templates,
            &rules_json,
        ) {
            mapgen::CompileOutcome::Error { diagnostics } => {
                serde_json::json!({"seed":seed,"stage":"generation","diagnostics":diagnostics})
            }
            mapgen::CompileOutcome::Ok { result } => {
                let prepared = sim::encounter::PreparedMap::new(&result.map, &rules);
                match sim::map_analysis::admit_skirmish(
                    &prepared.queries(&result.map, &result.sites),
                    &rules,
                    &players,
                ) {
                    Ok(admitted) => {
                        serde_json::json!({"seed":seed,"stage":"admitted","extent_m":request.extent_m(),"sites":admitted.sites.skirmish,"journeys":admitted.journeys})
                    }
                    Err(message) => {
                        serde_json::json!({"seed":seed,"stage":"navigation","message":message})
                    }
                }
            }
        };
        println!("{} {}ms", result, started.elapsed().as_millis());
    }
}
