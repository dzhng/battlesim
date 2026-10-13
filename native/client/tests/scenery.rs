//! The replay places the ground's scenery from the bundles' finest meshes,
//! as the page does, and a felled tree's stump row draws the presentation's
//! stump: its kind's trunk cut at the biome's height. Reads the fixture
//! run's world and the scenery bundles (Git LFS).
use std::path::Path;

use native_client::bundle::Library;
use native_client::capture::Replay;
use presentation::assets::RUNTIME;
use presentation::static_world::ground::mesh::VERTEX_FLOATS;
use presentation::static_world::ground::walk::Draws;

const RUN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/run/paris-corner"
);
const GAME: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/game.json");

#[test]
fn a_felled_trees_stump_is_its_trunk_cut_at_the_biomes_height() {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let replay = Replay::open(Path::new(RUN), Path::new(GAME), &library).unwrap();
    let presentation = replay.presentation();
    let trees = &presentation
        .ground()
        .expect("the replay sets the ground")
        .biome
        .trees;
    let world = presentation.world();
    let mut stumps = 0;
    for (row, a) in world.appearances.iter().enumerate() {
        let stump = presentation.stump_mesh(row as u32);
        if a.draws != Draws::Stump as u32 {
            assert!(stump.is_none(), "row {row} draws {}", a.draws);
            continue;
        }
        let name = &world.strings[a.name as usize..(a.name + a.name_len) as usize];
        // Only a tree is felled: a shrub or a piece of dressing leaves no stump.
        let tree = trees.species.iter().any(|s| s.appearance == name);
        assert_eq!(stump.is_some(), tree, "{name}");
        let Some(stump) = stump else { continue };
        let vertices = stump.as_chunks::<VERTEX_FLOATS>().0;
        let top = vertices.iter().map(|v| f64::from(v[2])).fold(0.0, f64::max);
        let reach = vertices
            .iter()
            .map(|v| f64::from(v[0]).hypot(f64::from(v[1])))
            .fold(0.0, f64::max);
        let cut_m = trees.felled.stump_height_m;
        assert!((top - cut_m).abs() < 1e-6, "{name} is cut at {top} m");
        assert!(
            reach > 0.05 && reach < 1.0,
            "{name}'s trunk is {reach} m round"
        );
        stumps += 1;
    }
    assert_eq!(
        stumps,
        trees.species.len(),
        "every tree kind leaves a stump"
    );
}
