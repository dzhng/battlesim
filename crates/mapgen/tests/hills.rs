//! Generated hills: plains maps get large sections of rolling rises, a
//! city none; they are seeded, mirrored half for half, capped in height,
//! drivable, and clear of the towns.
use contract::map::{relief_height, Relief};
use mapgen::layout::{GenerationRequest, MapSize, MapType, PresetDefinitions, GENERATOR_VERSION};
use mapgen::{CompileLimits, GenerateOutcome, MapPlan};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");

fn plan(map_type: MapType, size: MapSize, seed: u64) -> MapPlan {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let presets = PresetDefinitions::from_json(PRESETS).unwrap();
    let request = GenerationRequest {
        generator_version: GENERATOR_VERSION.into(),
        preset_revision: presets.revision.clone(),
        seed: seed.into(),
        template_catalog_hash: catalogue.hash().into(),
        map_type,
        size,
        entries_per_side: 1,
        profile: contract::generation::GenerationProfile::Standard,
        region: None,
        limits: CompileLimits {
            max_authored_parts: 250_000,
            max_bay_positions: 600_000,
            max_ground_points: 200_000,
        },
    };
    let rules = sim::fixtures::test_rules().to_string();
    match mapgen::generate_plan(
        &serde_json::to_string(&request).unwrap(),
        PRESETS,
        TEMPLATES,
        &rules,
    ) {
        GenerateOutcome::Ok { plan } => *plan,
        GenerateOutcome::Error { diagnostics } => {
            panic!("{map_type:?} {size:?} seed {seed}: {diagnostics:?}")
        }
    }
}

#[test]
fn plains_roll_cities_stay_flat() {
    let presets: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    let cap = presets["hills"]["max_height_m"].as_f64().unwrap();
    let plains = plan(MapType::Open, MapSize::Medium, 1);
    assert!(
        plains.relief.len() >= 8,
        "a plains map has hills: {} rises",
        plains.relief.len()
    );
    // Seeded: the same request draws the same hills.
    let again = plan(MapType::Open, MapSize::Medium, 1);
    assert_eq!(
        serde_json::to_string(&plains.relief).unwrap(),
        serde_json::to_string(&again.relief).unwrap()
    );
    // Mirrored through the centre, rise for rise.
    let size = plains.size;
    for pair in plains.relief.chunks(2) {
        let [Relief::Ridge { center: a, .. }, Relief::Ridge { center: b, .. }] = pair else {
            panic!("rises come in mirrored pairs: {pair:?}");
        };
        assert!((a[0] + b[0] - size[0]).abs() < 1e-9 && (a[1] + b[1] - size[1]).abs() < 1e-9);
    }
    // Capped, drivable, and off every town, on the height grid.
    let grid = plains.height_grid_m;
    let cutoff = libm::tan(plains.slope_cutoff_deg.to_radians());
    let (nx, ny) = ((size[0] / grid) as usize, (size[1] / grid) as usize);
    let h = |i: usize, j: usize| relief_height(&plains.relief, i as f64 * grid, j as f64 * grid);
    let mut highest: f64 = 0.0;
    for j in 0..ny {
        for i in 0..nx {
            let z = h(i, j);
            highest = highest.max(z);
            let slope = ((h(i + 1, j) - z) / grid).hypot((h(i, j + 1) - z) / grid);
            assert!(slope < cutoff, "a climbable slope at {i},{j}: {slope}");
            if z > 0.0 {
                let p = [i as f64 * grid, j as f64 * grid];
                assert!(
                    plains
                        .settlements
                        .iter()
                        .all(|s| !contract::ground::polygon_contains(&s.outline, p)),
                    "a hill under a town at {p:?}"
                );
            }
        }
    }
    assert!(
        highest > 3.0 && highest <= cap,
        "highest {highest} under {cap}"
    );
    assert!(plan(MapType::Metro, MapSize::Medium, 1).relief.is_empty());
}
