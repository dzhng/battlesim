//! No crop is drilled on a hill: every plot on rising ground is the biome's
//! surround meadow, where the same ground left flat is cut into crops.
use contract::map::MapDefinition;
use presentation::static_world::ground::biome::Biome;
use presentation::static_world::ground::{self, Ground, Tuning};
use serde_json::{json, Value};
use sim::world::WorldGeometry;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const HILL: [f64; 2] = [480.0, 390.0];

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

/// The kinds of the plots whose middle lies within `reach` of the hill's.
fn kinds_on_the_hill(g: &Ground, biome: &Biome, reach: f64) -> Vec<String> {
    g.plots
        .plots
        .iter()
        .filter(|p| {
            let n = p.outline.len() / 2;
            let c = (0..n).fold([0.0, 0.0], |c, i| {
                [
                    c[0] + p.outline[2 * i] / n as f64,
                    c[1] + p.outline[2 * i + 1] / n as f64,
                ]
            });
            (c[0] - HILL[0]).hypot(c[1] - HILL[1]) < reach
        })
        .map(|p| biome.plots[p.kind].name.clone())
        .collect()
}

#[test]
fn no_crop_is_drilled_on_a_hill() {
    let hill = json!([{ "kind": "ridge", "center": HILL, "peak_m": 6.0, "radius_m": 110.0 }]);
    let mut cropped = false;
    for seed in 0..8 {
        let (flat, biome) = ground(json!([]), seed);
        let surround = &biome.field_rules.surround_kind;
        cropped |= kinds_on_the_hill(&flat, &biome, 90.0)
            .iter()
            .any(|k| k != surround);
        let (hilly, biome) = ground(hill.clone(), seed);
        let on = kinds_on_the_hill(&hilly, &biome, 90.0);
        assert!(!on.is_empty(), "seed {seed}: plots lie on the hill");
        assert!(
            on.iter().all(|k| k == surround),
            "seed {seed}: every plot on the hill is {surround}, not {on:?}"
        );
    }
    assert!(cropped, "the same ground left flat is cut into crops");
}
