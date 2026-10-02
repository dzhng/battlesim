//! Generate every type × size over a run of seeds and report what came out:
//! refusals by feature, composition, roads and transit of the layout, its
//! river and bridges beside the maps without one, then what the parcel pass
//! built on it, the street furniture placed among that, and what the
//! compiled map costs.
//!
//!   cargo run -p mapgen --release --example layout_sweep -- [--seeds 10]
//!       [--pictures 3] [--out <dir>] [--only metro:small] [--layout]
//!       [--set /classes/city/area_share=[0.3,0.4]]...
//!
//! `--set` edits one preset value (a JSON pointer) for a tuning trial.
//! `--layout` stops before the parcel pass. `--out` writes `sweep.json`, the
//! tables, and for each pictured seed the whole map, one district of each
//! kind and, where there is a river, its first bridge and the settlement
//! nearest the water, as SVG.
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::{
    generate_layout, measure, GenerationRequest, LayoutMetrics, MapSize, MapType,
    PresetDefinitions, GENERATOR_VERSION,
};
use mapgen::parcels::fill_districts;
use mapgen::street_props::place_street_props;
use mapgen::CompileLimits;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::time::Instant;

#[path = "../../sim/examples/common/instructions.rs"]
mod instructions;
use instructions::instructions;

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

/// The slower of the top and bottom edges' journeys to the centre.
fn to_centre(metrics: &LayoutMetrics) -> f64 {
    let transit = &metrics.transit;
    [&transit.top, &transit.bottom]
        .into_iter()
        .map(|journey| journey.as_ref().map_or(f64::INFINITY, |j| j.elapsed_s))
        .fold(0.0, f64::max)
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

/// What one filled, compiled plan came to.
struct Scale {
    buildings: f64,
    /// Building parts and street furniture together: every authored body.
    parts: f64,
    furniture: f64,
    bays: f64,
    street_km: f64,
    streets: f64,
    ground_points: f64,
    built_share: f64,
    map_mib: f64,
    instructions_g: f64,
    millis: f64,
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
    let layout_only = arguments.iter().any(|argument| argument == "--layout");

    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");
    let mut source: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(format!(
        "{fixtures}/map-presets.json"
    ))?)?;
    for edit in values("--set") {
        let (pointer, value) = edit.split_once('=').ok_or("--set takes pointer=json")?;
        *source
            .pointer_mut(pointer)
            .ok_or("unknown preset pointer")? = serde_json::from_str(value)?;
    }
    let presets =
        PresetDefinitions::from_json(&source.to_string()).map_err(|e| format!("{e:?}"))?;
    let catalogue = TemplateGeometryCatalog::new(serde_json::from_str(&std::fs::read_to_string(
        format!("{fixtures}/prototype-building-templates.json"),
    )?)?)?;
    // The unit and prop catalog, out of the resolved view the browser reads.
    let view: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(format!(
        "{fixtures}/catalog.json"
    ))?)?;
    let catalog: contract::catalog::Catalog = serde_json::from_value(view["documents"].clone())?;
    if let Some(out) = &out {
        std::fs::create_dir_all(out)?;
    }

    let mut layout_table = String::from(
        "| cell | ok | refused | settlements | urban % | main % of urban | forest % | forest km² | woods | road km | track km | loops | exits | central crossroads | side-to-side road | top or bottom to centre s | bottom to top s | bottom to top km | approaches top/bottom | ground points | ms median/max |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    let mut river_table = String::from(
        "| cell | ok | refused | with a river | river km | water width m | bridges | bridges top/bottom | connected | fair | approach in both halves | slowest top or bottom to centre s, river / none | layout instructions M median, river / none |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    let mut scale_table = String::from(
        "| cell | ok | refused | buildings | authored bodies | of them street furniture | bay positions | street km | street strokes | ground points | built ground % | generate ms median/max | generate + compile instructions G median/max | map.json MiB |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    let mut records = Vec::new();
    for map_type in MapType::ALL {
        for size in MapSize::ALL {
            let cell = format!("{}:{}", map_type.name(), size.name());
            if only.as_ref().is_some_and(|only| *only != cell) {
                continue;
            }
            let mut refused: BTreeMap<String, u32> = BTreeMap::new();
            let mut passed: Vec<LayoutMetrics> = Vec::new();
            // Millions of instructions each accepted layout took.
            let mut layout_work: Vec<f64> = Vec::new();
            let mut scales: Vec<Scale> = Vec::new();
            let mut millis: Vec<f64> = Vec::new();
            for seed in 1..=seeds {
                let request = GenerationRequest {
                    generator_version: GENERATOR_VERSION.into(),
                    preset_revision: presets.revision.clone(),
                    seed: seed.into(),
                    template_catalog_hash: catalogue.hash().into(),
                    map_type,
                    size,
                    limits: CompileLimits {
                        max_authored_parts: 1_000_000,
                        max_bay_positions: 100_000_000,
                        max_ground_points: 10_000_000,
                    },
                };
                let (started, before) = (Instant::now(), instructions());
                let layout = generate_layout(&request, &presets);
                millis.push(started.elapsed().as_secs_f64() * 1000.0);
                let layout_spent = instructions().zip(before).map_or(0, |(a, b)| a - b);
                let result = layout.and_then(|layout| {
                    let metrics = measure(&layout, &presets);
                    let plan = if layout_only {
                        layout
                    } else {
                        let mut plan = fill_districts(layout, &request, &catalogue, &presets)?;
                        let props =
                            place_street_props(&plan, &request, &catalogue, &catalog, &presets)?;
                        plan.props.extend(props);
                        plan
                    };
                    Ok((plan, metrics))
                });
                let generate_ms = started.elapsed().as_secs_f64() * 1000.0;
                match result {
                    Ok((plan, metrics)) => {
                        let name = format!("{}-{}-s{seed}", map_type.name(), size.name());
                        let title = format!(
                            "{map_type:?} {size:?} {0}×{0} km, seed {seed}",
                            size.extent_m() / 1000.0
                        );
                        let mut record = serde_json::json!({
                            "type": map_type, "size": size, "seed": seed, "metrics": metrics,
                        });
                        if !layout_only {
                            let filled = measure(&plan, &presets);
                            let compiled = mapgen::lower(
                                &mapgen::CompileRequest::generated(&request, plan.clone()),
                                &catalogue,
                            )
                            .map_err(|errors| format!("{cell} seed {seed}: {errors:?}"))?;
                            let spent = instructions().zip(before).map_or(0, |(a, b)| a - b);
                            let scale = Scale {
                                buildings: plan.buildings.len() as f64,
                                parts: f64::from(compiled.report.authored_parts),
                                furniture: plan.props.len() as f64,
                                bays: compiled.report.bay_positions as f64,
                                street_km: filled.roads.street_km,
                                streets: plan
                                    .surfaces
                                    .iter()
                                    .filter(|area| {
                                        area.kind == contract::map::SurfaceKind::Road
                                            && matches!(
                                                area.shape,
                                                contract::ground::GroundShape::Stroke { .. }
                                            )
                                    })
                                    .count() as f64,
                                ground_points: compiled.report.ground_points as f64,
                                built_share: metrics.urban_share * 100.0,
                                map_mib: serde_json::to_vec(&compiled.map)?.len() as f64
                                    / 1048576.0,
                                instructions_g: spent as f64 / 1e9,
                                millis: generate_ms,
                            };
                            record["scale"] = serde_json::json!({
                                "buildings": scale.buildings, "parts": scale.parts,
                                "street_props": scale.furniture,
                                "bay_positions": scale.bays, "street_km": scale.street_km,
                                "street_strokes": scale.streets, "ground_points": scale.ground_points,
                                "map_mib": scale.map_mib, "instructions_g": scale.instructions_g,
                                "generate_ms": scale.millis,
                            });
                            scales.push(scale);
                        }
                        if let Some(out) = out.as_ref().filter(|_| seed <= pictures) {
                            let picture = |crop: Option<&str>| {
                                mapgen::inspect::svg(
                                    &plan,
                                    &catalogue,
                                    &title,
                                    &measure(&plan, &presets),
                                    crop,
                                )
                            };
                            std::fs::write(out.join(format!("{name}.svg")), picture(None)?)?;
                            // The first bridge, where a road crosses, and the settlement
                            // nearest the water.
                            if let Some(river) = plan.rivers.first() {
                                if !plan.bridges.is_empty() {
                                    std::fs::write(
                                        out.join(format!("{name}-bridge-0.svg")),
                                        picture(Some("bridge-0"))?,
                                    )?;
                                }
                                let from_water = |settlement: &mapgen::SettlementPlan| {
                                    settlement
                                        .outline
                                        .iter()
                                        .map(|p| -river.inside(*p))
                                        .fold(f64::INFINITY, f64::min)
                                };
                                let riverside = plan
                                    .settlements
                                    .iter()
                                    .min_by(|a, b| from_water(a).total_cmp(&from_water(b)));
                                if let Some(settlement) = riverside {
                                    std::fs::write(
                                        out.join(format!("{name}-riverside.svg")),
                                        picture(Some(&settlement.id))?,
                                    )?;
                                }
                            }
                            // One district of each kind, close enough to read its parcels.
                            let mut drawn = BTreeSet::new();
                            for district in plan.settlements.iter().flat_map(|s| &s.districts) {
                                if !layout_only && drawn.insert(&district.kind) {
                                    std::fs::write(
                                        out.join(format!("{name}-{}.svg", district.kind)),
                                        picture(Some(&district.id))?,
                                    )?;
                                }
                            }
                        }
                        records.push(record);
                        passed.push(metrics);
                        layout_work.push(layout_spent as f64 / 1e6);
                    }
                    Err(errors) => {
                        for error in &errors {
                            eprintln!(
                                "{cell} seed {seed}: {} at {}: {}",
                                error.feature.as_deref().unwrap_or("-"),
                                error.location,
                                error.message
                            );
                            let feature = error.feature.clone().unwrap_or_default();
                            // Settlement, district and road refusals name an index.
                            let feature = feature
                                .split(['-', '/'])
                                .next()
                                .unwrap_or_default()
                                .to_string();
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
            let refusals = if refusals.is_empty() {
                "-".into()
            } else {
                refusals.join(", ")
            };
            writeln!(
                layout_table,
                "| {cell} | {}/{seeds} | {refusals} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0}% | {:.0}% | {} | {} | {} | {} / {} | {} | {:.1}/{:.1} |",
                passed.len(),
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
                100.0 * passed.iter().filter(|m| m.roads.centre_roads >= 4).count() as f64
                    / passed.len().max(1) as f64,
                100.0 * passed.iter().filter(|m| m.transit.east_west.is_some()).count() as f64
                    / passed.len().max(1) as f64,
                each(to_centre, 0),
                each(|m| m.transit.top_bottom.as_ref().map_or(f64::INFINITY, |j| j.elapsed_s), 0),
                each(|m| m.transit.top_bottom.as_ref().map_or(f64::INFINITY, |j| j.route_m / 1000.0), 1),
                each(|m| m.approaches_top as f64, 0),
                each(|m| m.approaches_bottom as f64, 0),
                each(|m| m.ground_points as f64, 0),
                millis[millis.len() / 2],
                millis[millis.len() - 1],
            )?;
            // Maps with a river beside those without: [river, none].
            let wet = |m: &LayoutMetrics| m.river.rivers > 0;
            let rivers: Vec<&LayoutMetrics> = passed.iter().filter(|m| wet(m)).collect();
            let of = |value: fn(&LayoutMetrics) -> f64, digits| {
                span(rivers.iter().map(|m| value(m)), digits)
            };
            let holds = |rule: fn(&LayoutMetrics) -> bool| {
                format!(
                    "{}/{}",
                    rivers.iter().filter(|m| rule(m)).count(),
                    rivers.len()
                )
            };
            let by_river = |value: &dyn Fn(usize) -> f64, median_of: bool| {
                [true, false].map(|river| {
                    let mut values: Vec<f64> = (0..passed.len())
                        .filter(|index| wet(&passed[*index]) == river)
                        .map(value)
                        .collect();
                    if values.is_empty() {
                        "-".to_string()
                    } else if median_of {
                        format!("{:.1}", median(&mut values))
                    } else {
                        format!("{:.0}", values.iter().copied().fold(0.0, f64::max))
                    }
                })
            };
            let transit = by_river(&|index| to_centre(&passed[index]), false);
            let work = by_river(&|index| layout_work[index], true);
            writeln!(
                river_table,
                "| {cell} | {}/{seeds} | {refusals} | {} | {} | {} | {} | {} / {} | {} | {} | {} | {} / {} | {} / {} |",
                passed.len(),
                rivers.len(),
                of(|m| m.river.top_km + m.river.bottom_km, 1),
                span(
                    rivers.iter().flat_map(|m| m.river.width_m),
                    0
                ),
                of(|m| (m.river.bridges_top + m.river.bridges_bottom) as f64, 0),
                of(|m| m.river.bridges_top as f64, 0),
                of(|m| m.river.bridges_bottom as f64, 0),
                holds(|m| m.roads.unconnected_settlements == 0 && m.roads.unbridged == 0),
                holds(|m| m.town.fair && m.forest.fair && m.river.fair),
                holds(|m| m.main_approach_top && m.main_approach_bottom),
                transit[0],
                transit[1],
                work[0],
                work[1],
            )?;
            if !layout_only {
                let each =
                    |value: fn(&Scale) -> f64, digits| span(scales.iter().map(value), digits);
                let mut work: Vec<f64> = scales.iter().map(|s| s.instructions_g).collect();
                let mut wall: Vec<f64> = scales.iter().map(|s| s.millis).collect();
                let (work_median, wall_median) = (median(&mut work), median(&mut wall));
                writeln!(
                    scale_table,
                    "| {cell} | {}/{seeds} | {refusals} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0}/{:.0} | {:.2}/{:.2} | {} |",
                    scales.len(),
                    each(|s| s.buildings, 0),
                    each(|s| s.parts, 0),
                    each(|s| s.furniture, 0),
                    each(|s| s.bays, 0),
                    each(|s| s.street_km, 0),
                    each(|s| s.streets, 0),
                    each(|s| s.ground_points, 0),
                    each(|s| s.built_share, 1),
                    wall_median,
                    wall.last().copied().unwrap_or(0.0),
                    work_median,
                    work.last().copied().unwrap_or(0.0),
                    each(|s| s.map_mib, 1),
                )?;
            }
        }
    }
    print!("{layout_table}\n{river_table}");
    if !layout_only {
        print!("\n{scale_table}");
    }
    if let Some(out) = &out {
        std::fs::write(out.join("sweep.json"), serde_json::to_vec_pretty(&records)?)?;
        std::fs::write(out.join("layout.md"), layout_table)?;
        std::fs::write(out.join("rivers.md"), river_table)?;
        if !layout_only {
            std::fs::write(out.join("scale.md"), scale_table)?;
        }
    }
    Ok(())
}
