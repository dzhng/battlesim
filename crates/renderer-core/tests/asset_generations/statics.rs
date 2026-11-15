//! Static placement and GPU owner contracts across complete admissions.
use super::*;

#[test]
fn unchanged_statics_are_carried_without_uploads_or_replacement_placement() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let scenery = bundle_bytes("static", 1, "summer", 4, &[]);
    let unit = bundle_bytes("static", 1, "default", 1, &[]);
    let initial: Art<'_> = &[("synthetic-scenery", &scenery)];
    let final_art: Art<'_> = &[("synthetic-scenery", &scenery), ("new-unit", &unit)];
    let mut config = runtime_config();
    config.grass_density = Some(1.0);
    let mut stage = Stage::new(&gpu, initial, config.clone(), Some(&scenery), &[]);
    let ready = stage.stage(&gpu, final_art, &[]);
    assert_eq!(ready.work().scenery_placements, 0);
    assert_eq!(ready.work().statics_bytes_uploaded, 0);
    stage.commit(&gpu, final_art, ready);
    let carried = stage.pixels(&gpu);
    let mut clean = Stage::new(&gpu, final_art, config, Some(&scenery), &[]);
    assert_eq!(
        carried,
        clean.pixels(&gpu),
        "carried statics differ from a clean final-catalog build"
    );
}

#[test]
fn changed_scenery_content_places_and_uploads_again() {
    let gpu = support::gpu().expect("a GPU is required for renderer admission tests");
    let original = bundle_bytes("static", 1, "summer", 4, &[]);
    let changed = bundle_bytes("static", 2, "summer", 4, &[]);
    let unit = bundle_bytes("static", 1, "default", 1, &[]);
    let mut config = runtime_config();
    config.grass_density = Some(1.0);
    let mut stage = Stage::new(
        &gpu,
        &[("synthetic-scenery", &original)],
        config,
        Some(&original),
        &[],
    );
    let ready = stage.stage(
        &gpu,
        &[("synthetic-scenery", &changed), ("new-unit", &unit)],
        &[],
    );
    assert_eq!(ready.work().scenery_placements, 1);
    assert!(ready.work().statics_bytes_uploaded > 0);
}
