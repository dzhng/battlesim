//! No crop is drilled on a hill and no field edge runs over one: every plot
//! on rising ground is the biome's surround meadow with no verge along its
//! edges, where the same ground left flat is cut into edged crops.
use contract::map::MapDefinition;
use presentation::static_world::ground::biome::Biome;
use presentation::static_world::ground::{self, Ground, Tuning};
use serde_json::{json, Value};
use sim::world::WorldGeometry;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const HILL: [f64; 2] = [480.0, 390.0];
const PEAK_M: f64 = 6.0;
const RADIUS_M: f64 = 110.0;

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).unwrap()
}

/// The ground of the river lab's map, with `relief`, under the summer biome
/// drawn from `seed`.
fn ground(relief: Value, seed: u32) -> (Ground, Biome) {
    let mut map: Value = serde_json::from_str(&read("fixtures/maps/river/map.json")).unwrap();
    map["relief"] = relief;
    let mut biome = Biome::load(&read("fixtures/biomes/summer.json"), None).unwrap();
    biome.seed = seed;
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let def: MapDefinition = serde_json::from_value(map).unwrap();
    let world = WorldGeometry::new(&def, &rules);
    let source = ground::source(
        &world,
        &def.extents(),
        &sim::world::export::layout_json(&rules.catalog),
    )
    .unwrap();
    let game: Value = serde_json::from_str(&read("fixtures/presentation.json")).unwrap();
    let tuning: Tuning = presentation::settings::PresentationSettings::from_value(&game)
        .unwrap()
        .tuning();
    (Ground::new(source, biome.clone(), tuning).unwrap(), biome)
}

/// The plots with a corner within the hill's metre contour, where the
/// ridge (`peak (1 - (d/r)^2)^2`) stands over a metre up, a grid cell in
/// from it so the sampled ground is up there too.
fn on_the_hill(g: &Ground) -> Vec<&presentation::static_world::ground::plots::Plot> {
    let contour = RADIUS_M * (1.0 - (1.0 / PEAK_M).sqrt()).sqrt() - 6.0;
    g.plots
        .plots
        .iter()
        .filter(|p| {
            p.outline
                .chunks(2)
                .any(|c| (c[0] - HILL[0]).hypot(c[1] - HILL[1]) < contour)
        })
        .collect()
}

#[test]
fn no_crop_is_drilled_and_no_field_edged_on_a_hill() {
    let hill = json!([{ "kind": "ridge", "center": HILL, "peak_m": PEAK_M, "radius_m": RADIUS_M }]);
    let mut cropped = false;
    for seed in 0..8 {
        let (flat, biome) = ground(json!([]), seed);
        let surround = &biome.field_rules.surround_kind;
        let level = on_the_hill(&flat);
        cropped |= level.iter().any(|p| &biome.plots[p.kind].name != surround);
        assert!(
            level.iter().all(|p| p.edged),
            "seed {seed}: flat plots are edged"
        );
        let (hilly, biome) = ground(hill.clone(), seed);
        let on = on_the_hill(&hilly);
        assert!(!on.is_empty(), "seed {seed}: plots lie on the hill");
        assert!(
            on.iter()
                .all(|p| &biome.plots[p.kind].name == surround && !p.edged),
            "seed {seed}: every plot on the hill is unedged {surround}"
        );
    }
    assert!(cropped, "the same ground left flat is cut into crops");
}
