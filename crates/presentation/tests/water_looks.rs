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
    let game: Value = serde_json::from_str(&read("fixtures/game.json")).unwrap();
    let tuning: Tuning = serde_json::from_value(game["presentation"].clone()).unwrap();
    Ground::new(source, biome, tuning).unwrap()
}

/// The look every water cell of `g` draws.
fn looks(g: &Ground) -> Vec<usize> {
    let cells = g.water_cells();
    assert!(!cells.is_empty(), "the map has water");
    cells
        .chunks(CELL_FLOATS)
        .map(|c| c[LOOK] as usize)
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
        let want = water::look(seed, 0, &biome.water.rivers);
        picked.insert(want);
        let plain = looks(&ground(&plain, biome.clone()));
        assert!(
            plain.iter().all(|&l| l == want),
            "seed {seed}: every cell of river 0 draws look {want}"
        );
        assert_eq!(looks(&ground(&busy, biome)), plain, "seed {seed}");
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
                water::look(seed, river, rivers),
                0,
                "seed {seed}, river {river}"
            );
        }
    }
}
