//! A small summer ground and stand-in scenery meshes for tests that need the
//! static world's scenery rows.
#![allow(dead_code)]
use presentation::presentation::Presentation;

/// Sets a 64 m summer ground, whose scenery `place_scenery` then places.
pub fn set_summer_ground(p: &mut Presentation) {
    use presentation::static_world::ground::{biome::Biome, tables::Grid, Source};
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/presentation.json")).unwrap();
    let biome = Biome::load(
        include_str!("../../../../fixtures/biomes/summer.json"),
        None,
    )
    .unwrap();
    p.set_ground(
        Source {
            map: [0.0, 0.0, 64.0, 64.0],
            playable: [0.0, 0.0, 64.0, 64.0],
            grid: Grid {
                nx: 17,
                ny: 17,
                spacing: 4.0,
                page_size: 16,
                ..Default::default()
            },
            ..Default::default()
        },
        biome,
        presentation::settings::PresentationSettings::from_value(&game)
            .unwrap()
            .tuning(),
    )
    .unwrap();
}

/// A stand-in scenery kind's finest mesh in the renderer's world vertex: a
/// trunk of `radius` under a crown up to `height`.
pub fn scenery_mesh(radius: f32, height: f32) -> Vec<f32> {
    [
        (radius, 0.0, 0.0),
        (-radius, 0.0, 0.0),
        (radius, 0.0, 1.0),
        (-radius, 0.0, 1.0),
        (radius * 2.0, 0.0, height),
        (-radius * 2.0, 0.0, height),
    ]
    .into_iter()
    .flat_map(|(x, y, z)| {
        [
            x,
            y,
            z,
            0.0,
            0.0,
            1.0,
            0.2,
            0.3,
            0.1,
            f32::from(z == height),
        ]
    })
    .collect()
}

/// Places the ground's scenery, every kind from `mesh`.
pub fn place_scenery(p: &mut Presentation, mesh: &[f32]) {
    let count = p.ground().unwrap().scenery_kinds().len();
    p.place_scenery(&vec![mesh; count]).unwrap();
}
