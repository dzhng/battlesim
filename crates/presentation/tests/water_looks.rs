//! Each river draws one of its biome's water looks: picked by weight from
//! the biome's seed and the river's index alone, as a tree stand is from its
//! cell, so no other feature of the map moves it, and never a look of no
//! weight.
use contract::map::MapDefinition;
use presentation::static_world::ground::biome::Biome;
use presentation::static_world::ground::water::{self, CELL_FLOATS};
use presentation::static_world::ground::{self, Ground, Tuning};
use serde_json::Value;
use sim::world::WorldGeometry;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const LOOK: usize = 4;

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).unwrap()
}

fn summer() -> Biome {
    Biome::load(&read("fixtures/biomes/summer.json"), None).unwrap()
}

/// The river lab's map, as its file has it.
fn river_map() -> Value {
    serde_json::from_str(&read("fixtures/maps/river/map.json")).unwrap()
}

/// The ground of map `map` under `biome`.
fn ground(map: &Value, biome: Biome) -> Ground {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    let def: MapDefinition = serde_json::from_value(map.clone()).unwrap();
    let world = WorldGeometry::new(&def, &rules);
    let source = ground::source(
        &world,
        &def.extents(),
        &sim::world::export::layout_json(&rules.catalog),
    )
    .unwrap();
    let game: Value = serde_json::from_str(&read("fixtures/presentation.json")).unwrap();
    let tuning: Tuning = serde_json::from_value(game.clone()).unwrap();
    Ground::new(source, biome, tuning).unwrap()
}

/// The look every river cell of `g` draws (the river looks' rows come
/// first, `rivers` of them).
fn looks(g: &Ground, rivers: usize) -> Vec<usize> {
    let cells = g.water_cells();
    assert!(!cells.is_empty(), "the map has water");
    cells
        .chunks(CELL_FLOATS)
        .map(|c| c[LOOK] as usize)
        .filter(|&l| l < rivers)
        .collect()
}

#[test]
fn a_river_draws_the_look_its_seed_and_index_pick_whatever_else_the_map_holds() {
    let plain = river_map();
    // The same river among more features (a swamp, another wood), and its
    // other features reordered.
    let mut busy = plain.clone();
    for list in ["surfaces", "forests"] {
        busy[list].as_array_mut().unwrap().reverse();
    }
    let wood = busy["forests"][0].clone();
    busy["forests"].as_array_mut().unwrap().push(wood);
    // A swamp on the open ground north of the river.
    busy["swamps"] = serde_json::json!([{
        "shape": { "kind": "polygon", "ring": [[440, 380], [560, 380], [560, 450], [440, 450]] },
        "depth_m": 0.6,
    }]);
    let mut picked = std::collections::BTreeSet::new();
    for seed in 0..12 {
        let mut biome = summer();
        biome.seed = seed;
        let want = water::look(seed, water::Kind::River, 0, &biome.water.rivers);
        picked.insert(want);
        let rivers = biome.water.rivers.len();
        let plain = looks(&ground(&plain, biome.clone()), rivers);
        assert!(
            !plain.is_empty(),
            "seed {seed}: the river draws a river look"
        );
        assert!(
            plain.iter().all(|&l| l == want),
            "seed {seed}: every cell of river 0 draws look {want}"
        );
        assert_eq!(looks(&ground(&busy, biome), rivers), plain, "seed {seed}");
    }
    assert!(picked.len() > 1, "the seeds pick more than one look");
}

#[test]
fn a_look_of_no_weight_is_never_drawn() {
    let mut biome = summer();
    let rivers = &mut biome.water.rivers;
    assert!(
        rivers.len() > 1,
        "the biome dresses rivers in several looks"
    );
    rivers[0].weight = 0.0;
    for seed in 0..500 {
        for river in 0..8 {
            assert_ne!(
                water::look(seed, water::Kind::River, river, rivers),
                0,
                "seed {seed}, river {river}"
            );
        }
    }
}

#[test]
fn a_swamp_draws_a_swamp_look_its_seed_and_index_pick_whatever_else_the_map_holds() {
    let swamp = |ring: serde_json::Value| serde_json::json!({ "shape": { "kind": "polygon", "ring": ring }, "depth_m": 0.6 });
    let mut one = river_map();
    one["swamps"] = serde_json::json!([swamp(serde_json::json!([
        [440, 380],
        [560, 380],
        [560, 450],
        [440, 450]
    ]))]);
    // The same swamp with another after it, and the woods reordered.
    let mut two = one.clone();
    two["swamps"]
        .as_array_mut()
        .unwrap()
        .push(swamp(serde_json::json!([
            [100, 380],
            [200, 380],
            [200, 450],
            [100, 450]
        ])));
    two["forests"].as_array_mut().unwrap().reverse();
    let mut picked = std::collections::BTreeSet::new();
    for seed in 0..12 {
        let mut biome = summer();
        biome.seed = seed;
        let (rivers, swamps) = (biome.water.rivers.len(), biome.water.swamps.len());
        let want = rivers + water::look(seed, water::Kind::Swamp, 0, &biome.water.swamps);
        picked.insert(want);
        // The first swamp's cells: those with a swamp look, west of x 300 in
        // the map with two.
        let first = |g: &Ground| -> Vec<usize> {
            let cells = g.water_cells();
            cells
                .chunks(CELL_FLOATS)
                .filter(|c| (rivers..rivers + swamps).contains(&(c[LOOK] as usize)) && c[0] > 300.0)
                .map(|c| c[LOOK] as usize)
                .collect()
        };
        let alone = first(&ground(&one, biome.clone()));
        assert!(!alone.is_empty(), "seed {seed}: the swamp has water cells");
        assert!(
            alone.iter().all(|&l| l == want),
            "seed {seed}: every cell of swamp 0 draws look {want}"
        );
        assert_eq!(first(&ground(&two, biome)), alone, "seed {seed}");
    }
    assert!(picked.len() > 1, "the seeds pick more than one swamp look");
}

#[test]
fn no_crop_stands_in_a_swamp_its_plots_are_the_ground_its_look_floods() {
    let mut map = river_map();
    let ring = [
        [440.0, 380.0],
        [560.0, 380.0],
        [560.0, 450.0],
        [440.0, 450.0],
    ];
    map["swamps"] =
        serde_json::json!([{ "shape": { "kind": "polygon", "ring": ring }, "depth_m": 0.6 }]);
    let inside =
        |[x, y]: [f64; 2]| x > ring[0][0] && x < ring[1][0] && y > ring[0][1] && y < ring[2][1];
    let mut grounds = std::collections::BTreeSet::new();
    for seed in 0..12 {
        let mut biome = summer();
        biome.seed = seed;
        let looks = &biome.water.swamps;
        let want = looks[water::look(seed, water::Kind::Swamp, 0, looks)]
            .ground
            .clone()
            .unwrap();
        grounds.insert(want.clone());
        let g = ground(&map, biome.clone());
        let flooded: Vec<_> = g
            .plots
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
                inside(c)
            })
            .map(|p| biome.plots[p.kind].name.clone())
            .collect();
        assert!(!flooded.is_empty(), "seed {seed}: plots lie in the swamp");
        assert!(
            flooded.iter().all(|k| *k == want),
            "seed {seed}: every plot in the swamp is {want}, not {flooded:?}"
        );
    }
    assert!(grounds.len() > 1, "the seeds flood more than one ground");
}

#[test]
fn a_road_across_a_swamp_stands_dry_as_the_simulation_keeps_it() {
    let mut map = river_map();
    // Across the country road (x 60, 9 m wide).
    map["swamps"] = serde_json::json!([{
        "shape": { "kind": "polygon", "ring": [[20, 380], [100, 380], [100, 440], [20, 440]] },
        "depth_m": 0.6,
    }]);
    let g = ground(&map, summer());
    let water = g.standing_water();
    assert!(
        water.at(30.0, 410.0).is_some_and(|w| w.depth_m > 0.3),
        "the swamp beside the road holds water"
    );
    for y in [385.0, 410.0, 435.0] {
        assert_eq!(water.at(60.0, y), None, "the road at y {y} is dry");
    }
}
