//! Generate every type × size over a run of seeds and report what came out:
//! refusals by feature, composition, roads, transit and generation time.
//!
//!   cargo run -p mapgen --release --example layout_sweep -- [--seeds 10]
//!       [--pictures 3] [--out <dir>] [--only metro:small]
//!       [--set /classes/city/area_share=[0.3,0.4]]...
//!
//! `--set` edits one preset value (a JSON pointer) for a tuning trial.
//! `--out` writes `sweep.json` and an SVG per pictured seed.
use mapgen::layout::{
    generate_layout, measure, GenerationRequest, LayoutMetrics, MapSize, MapType,
    PresetDefinitions, GENERATOR_VERSION,
};
use mapgen::CompileLimits;
use std::collections::BTreeMap;
use std::time::Instant;

fn span(values: impl Iterator<Item = f64>, digits: usize) -> String {
    let values: Vec<f64> = values.collect();
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if values.is_empty() {
        "-".into()
    } else {
        format!("{low:.digits$}–{high:.digits$}")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let values = |flag: &str| -> Vec<&String> {
        arguments
            .iter()
            .enumerate()
            .filter(|(_, argument)| *argument == flag)
            .filter_map(|(index, _)| arguments.get(index + 1))
            .collect()
    };
    let number = |flag: &str, default: u64| {
        values(flag)
            .first()
            .map_or(Ok(default), |value| value.parse::<u64>())
    };
    let seeds = number("--seeds", 10)?;
    let pictures = number("--pictures", 3)?;
    let out = values("--out").first().map(std::path::PathBuf::from);
    let only = values("--only").first().map(|cell| cell.to_string());

    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/map-presets.json"
    );
    let mut source: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file)?)?;
    for edit in values("--set") {
        let (pointer, value) = edit.split_once('=').ok_or("--set takes pointer=json")?;
        *source
            .pointer_mut(pointer)
            .ok_or("unknown preset pointer")? = serde_json::from_str(value)?;
    }
    let presets =
        PresetDefinitions::from_json(&source.to_string()).map_err(|e| format!("{e:?}"))?;
    if let Some(out) = &out {
        std::fs::create_dir_all(out)?;
    }

    println!(
        "| cell | ok | refused | settlements | urban % | main % of urban | forest % | forest km² | woods | road km | track km | loops | exits | central crossroads | worst transit s | approaches top/bottom | ground points | ms median/max |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let mut records = Vec::new();
    for map_type in MapType::ALL {
        for size in MapSize::ALL {
            let cell = format!("{}:{}", map_type.name(), size.name());
            if only.as_ref().is_some_and(|only| *only != cell) {
                continue;
            }
            let mut refused: BTreeMap<String, u32> = BTreeMap::new();
            let mut passed: Vec<LayoutMetrics> = Vec::new();
            let mut millis: Vec<f64> = Vec::new();
            for seed in 1..=seeds {
                let request = GenerationRequest {
                    generator_version: GENERATOR_VERSION.into(),
                    preset_revision: presets.revision.clone(),
                    seed: seed.into(),
                    template_catalog_hash: String::new(),
                    map_type,
                    size,
                    limits: CompileLimits {
                        max_authored_parts: 0,
                        max_bay_positions: 0,
                        max_ground_points: 1_000_000,
                    },
                };
                let started = Instant::now();
                let result = generate_layout(&request, &presets);
                millis.push(started.elapsed().as_secs_f64() * 1000.0);
                match result {
                    Ok(plan) => {
                        let metrics = measure(&plan, &presets);
                        if let Some(out) = out.as_ref().filter(|_| seed <= pictures) {
                            let name = format!("{}-{}-s{seed}", map_type.name(), size.name());
                            let title = format!(
                                "{map_type:?} {size:?} {0}×{0} km, seed {seed}",
                                size.extent_m() / 1000.0
                            );
                            std::fs::write(
                                out.join(format!("{name}.svg")),
                                mapgen::inspect::svg(&plan, &title, &metrics),
                            )?;
                        }
                        records.push(serde_json::json!({
                            "type": map_type, "size": size, "seed": seed, "metrics": metrics,
                        }));
                        passed.push(metrics);
                    }
                    Err(errors) => {
                        for error in &errors {
                            eprintln!("{cell} seed {seed}: {}", error.message);
                            let feature = error.feature.clone().unwrap_or_default();
                            // Settlement and road refusals name an index.
                            let feature = feature.split('-').next().unwrap_or_default().to_string();
                            *refused.entry(feature).or_default() += 1;
                        }
                        records.push(serde_json::json!({
                            "type": map_type, "size": size, "seed": seed, "refused": errors,
                        }));
                    }
                }
            }
            millis.sort_by(f64::total_cmp);
            let each =
                |value: fn(&LayoutMetrics) -> f64, digits| span(passed.iter().map(value), digits);
            let refusals: Vec<String> = refused
                .iter()
                .map(|(feature, count)| format!("{feature} {count}"))
                .collect();
            println!(
                "| {cell} | {}/{seeds} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0}% | {} | {} / {} | {} | {:.1}/{:.1} |",
                passed.len(),
                if refusals.is_empty() { "-".into() } else { refusals.join(", ") },
                each(|m| m.settlements.values().sum::<usize>() as f64, 0),
                each(|m| m.urban_share * 100.0, 1),
                each(|m| m.main_settlement_share * 100.0, 0),
                each(|m| m.forest_share * 100.0, 1),
                each(|m| (m.forest.top_m2 + m.forest.bottom_m2) / 1e6, 1),
                each(|m| m.woods as f64, 0),
                each(|m| m.roads.country_road_km, 0),
                each(|m| m.roads.dirt_track_km, 0),
                each(|m| m.roads.loops as f64, 0),
                each(|m| m.roads.edge_exits as f64, 0),
                100.0 * passed.iter().filter(|m| m.roads.hub_roads >= 4).count() as f64
                    / passed.len().max(1) as f64,
                each(|m| m.transit.iter().map(|e| e.elapsed_s).fold(0.0, f64::max), 0),
                each(|m| m.approaches_top as f64, 0),
                each(|m| m.approaches_bottom as f64, 0),
                each(|m| m.ground_points as f64, 0),
                millis[millis.len() / 2],
                millis[millis.len() - 1],
            );
        }
    }
    if let Some(out) = &out {
        std::fs::write(out.join("sweep.json"), serde_json::to_vec_pretty(&records)?)?;
    }
    Ok(())
}
