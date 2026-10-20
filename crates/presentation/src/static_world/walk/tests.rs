use super::*;
use crate::static_world::buildings::Placed;
use crate::static_world::index::BoundsIndex;
use crate::static_world::library::Library;
use crate::view::Camera;
use crate::world::BuildingStyle;

fn town(remote: usize) -> BuildingWalk {
    // Three distinct state rows, each available at every tier.
    let header = serde_json::json!({
        "content": {
            "kits": [{"appearance": "test"}], "modules": [{"kit": 0, "module": "box"}],
            "templates": [{"id": "box", "states": {
                "intact": {"first": 0, "count": 1}, "ruin": {"first": 1, "count": 1}, "gutted": {"first": 2, "count": 1}}}],
            "rows": {"module": {"$view": 0}, "transform": {"$view": 1}, "tiers": {"$view": 2}, "tint": {"$view": 3}}
        },
        "views": [["u16", 0, 3], ["f32", 6, 21], ["u8", 90, 3], ["u8", 93, 9]]
    });
    let head = serde_json::to_vec(&header).unwrap();
    let mut body = vec![0u8; 6];
    for z in [0f32, 1., 2.] {
        for v in [0., 0., z, 0., 1., 1., 1.] {
            body.extend(v.to_le_bytes());
        }
    }
    body.extend([15; 3]);
    body.extend([200; 9]);
    let mut bytes: Vec<_> = [
        crate::assets::LIBRARY.0,
        crate::assets::LIBRARY.1,
        head.len() as u32,
        body.len() as u32,
    ]
    .into_iter()
    .flat_map(u32::to_le_bytes)
    .collect();
    bytes.extend(head);
    bytes.extend(body);
    let style = BuildingStyle {
        lod_px_per_m: [10., 3., 1.],
        chunk_m: 64.,
        pool_records: 2,
        tint_jitter: 0.,
    };
    let buildings = Buildings::new(
        Library::decode(&bytes).unwrap(),
        &[Some([-5., -5., 0., 5., 5., 15.])],
        (0..remote + 8)
            .map(|i| Placed {
                owner: i as u32,
                template: "box".into(),
                frame: [
                    if i < 8 {
                        i as f64 * 64. - 200.
                    } else {
                        10000. + i as f64 * 128.
                    },
                    0.,
                    -20.,
                    0.3,
                ],
            })
            .collect(),
        &style,
    )
    .unwrap();
    BuildingWalk::new(
        buildings,
        style.lod_px_per_m,
        style.chunk_m,
        style.pool_records,
    )
}

#[test]
fn spatial_query_preserves_building_lists_residency_and_known_ruins() {
    let mut baseline = Vec::new();
    for remote in [64, 1024, 4096] {
        let mut sample = 0;
        let (mut indexed, mut brute) = (town(remote), town(remote));
        for state in 0..4 {
            let fallen = match state {
                1 => vec![(0, 1), (3, 2)],
                2 => (0..remote + 8).map(|i| (i, 1 + i % 2)).collect(),
                _ => vec![],
            };
            assert!(indexed.set_fallen(&fallen).is_empty());
            assert!(brute.set_fallen(&fallen).is_empty());
            brute.chunk_index = BoundsIndex::new(
                brute
                    .chunks
                    .iter()
                    .map(|_| [-1e12, -1e12, -1e12, 1e12, 1e12, 1e12]),
            );
            brute.ruin_index = BoundsIndex::new(
                brute
                    .ruins
                    .iter()
                    .map(|_| [-1e12, -1e12, -1e12, 1e12, 1e12, 1e12]),
            );
            for x in [-150., 0., 150.] {
                let camera = Camera {
                    eye: [x, -30., 0.],
                    target: [x, 100., -20.],
                    up: [0., 0., 1.],
                    fov_y: 0.8,
                    near: 1.,
                    size: [1280, 720],
                };
                let fixture: serde_json::Value = serde_json::from_str(include_str!(
                    "../../../tests/fixtures/shadow-vectors.json"
                ))
                .unwrap();
                let mut light: crate::light::LightSettings =
                    serde_json::from_value(fixture["lights"]["game"].clone()).unwrap();
                light.cascades.max_far_m = 100.;
                let terrain = crate::static_world::ground::partition::TerrainPartition::new(
                    &crate::static_world::ground::mesh::terrain(
                        &[
                            -300., -100., -20., 300., -100., -20., 300., 100., -20., -300., 100.,
                            -20.,
                        ],
                        &[0, 1, 2, 0, 2, 3],
                        None,
                    ),
                    crate::static_world::ground::partition::CELL_M,
                );
                let fit = crate::shadow::sun_shadow(&light, &camera, Some(&terrain));
                let shadow = CasterVolumes::new(&fit.casters);
                for shadow in [None, Some(&shadow)] {
                    let view = View::new(&camera);
                    assert_eq!(indexed.select(&view, shadow), brute.select(&view, shadow));
                    assert_eq!(indexed.lists.view, brute.lists.view);
                    assert_eq!(indexed.lists.cast, brute.lists.cast);
                    assert_eq!(indexed.counts, brute.counts);
                    let output = (indexed.lists.view.clone(), indexed.lists.cast.clone());
                    if remote == 64 {
                        baseline.push(output);
                    } else {
                        assert_eq!(
                            output, baseline[sample],
                            "remote growth changed local output"
                        );
                    }
                    sample += 1;
                    assert_eq!(&indexed.stats[..11], &brute.stats[..11]);
                    assert!(indexed.stats[11] < 150, "visited {}", indexed.stats[11]);
                }
            }
        }
    }
}
