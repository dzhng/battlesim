//! The renderer packs the actual admitted grass bundles, including far tiers.
#[path = "support/runtime_library.rs"]
mod runtime_library;
use presentation::static_world::files;
use presentation::static_world::ground::{
    grass::{Quality, LIMITS},
    Ground,
};
use renderer_core::bundle::Library;
use std::path::Path;

#[test]
fn every_summer_growth_row_fits_the_height_cap_with_its_catalog_kinds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = files::rules(&root, "sensors").unwrap();
    let ground = files::ground(&root, "sensors", &rules).unwrap();
    let ground = Ground::new(ground.source, ground.biome, ground.tuning).unwrap();
    let names = ground.grass_kinds();
    let library = Library::default();
    let error = renderer_core::grass::pack_shapes(&library, &names)
        .err()
        .unwrap();
    assert!(
        error.contains(&names[0]),
        "missing art refuses by name: {error}"
    );
    let library = runtime_library::runtime_library(&names).unwrap();
    let packed = renderer_core::grass::pack_shapes(&library, &names).unwrap();
    let params = ground
        .grass_params(&packed.facts, &LIMITS, &Quality { density: 1.0 })
        .unwrap();
    assert!(params.window[3] > 0.0);
}
