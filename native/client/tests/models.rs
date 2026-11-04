//! A corpse lies on the ground as the presentation crate poses it: its body,
//! skinned once by its row's palette, is a man lying down, not standing.
//! Reads the fixture run's world and the rifleman's bundle (Git LFS).
use std::path::Path;

use native_client::bundle::Library;
use native_client::capture::Replay;
use presentation::assets::RUNTIME;
use presentation::records::NONE;
use renderer_core::bundle::Shape;
use renderer_core::models::posed_mesh;

const RUN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/run/paris-corner"
);

fn heights(positions: &[f32]) -> (f32, f32) {
    let z = positions.as_chunks::<3>().0.iter().map(|p| p[2]);
    (
        z.clone().fold(f32::MAX, f32::min),
        z.fold(f32::MIN, f32::max),
    )
}

#[test]
fn a_corpse_lies_on_the_ground() {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let replay = Replay::open(Path::new(RUN), &library).unwrap();
    let presentation = replay.presentation();
    let world = presentation.world();
    let text = |at: u32, len: u32| &world.strings[at as usize..(at + len) as usize];
    let mut checked = 0;
    for (row, a) in world.appearances.iter().enumerate() {
        if a.lies == NONE || text(a.name, a.name_len) != "rifle_squad_active_a" {
            continue;
        }
        let bundle = library.bundle("rifle_squad_active_a").unwrap();
        let Shape::Skinned(tiers) = &bundle.shape else {
            panic!("a soldier is skinned")
        };
        let (_, standing) = heights(&tiers[0].positions);
        let palette = presentation.rest_palette(row as u32).unwrap();
        let (low, high) = heights(&posed_mesh(&tiers[0], &palette).positions);
        assert!(standing > 1.5, "the body stands {standing} m tall");
        assert!(high < 0.6, "way {} lies {high} m high", a.lies);
        assert!(low > -0.2, "way {} sinks to {low} m", a.lies);
        checked += 1;
    }
    assert!(checked > 0, "the fixture has a rifleman's corpse row");
}

/// Where a body is too small for its mesh, it draws as its card shows it: in
/// its far pose, standing, not in the bind pose the raw mesh holds.
#[test]
fn a_far_body_stands_in_its_far_pose() {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let replay = Replay::open(Path::new(RUN), &library).unwrap();
    let presentation = replay.presentation();
    let world = presentation.world();
    let row = world
        .appearances
        .iter()
        .position(|a| {
            a.lies == NONE
                && &world.strings[a.name as usize..(a.name + a.name_len) as usize]
                    == "rifle_squad_active_a"
        })
        .expect("a rifleman's body row");
    let bundle = library.bundle("rifle_squad_active_a").unwrap();
    let Shape::Skinned(tiers) = &bundle.shape else {
        panic!("a soldier is skinned")
    };
    let palette = presentation.rest_palette(row as u32).unwrap();
    let (low, high) = heights(&posed_mesh(&tiers[0], &palette).positions);
    assert!(high > 1.3, "the far pose stands {high} m tall");
    assert!(low.abs() < 0.2, "its feet are {low} m off the ground");
}
