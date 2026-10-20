use super::*;
use crate::static_world::index::BoundsIndex;
use crate::view::Camera;

#[test]
fn spatial_query_keeps_complete_scenery_lists_and_prints_after_restanding() {
    let sizes = [Size {
        height: 15.,
        radius: 5.,
    }];
    let mut baseline = Vec::new();
    for remote in [64, 1024, 4096] {
        let mut sample = 0;
        let placed: Vec<f32> = (0..remote + 8)
            .flat_map(|i| {
                let x = if i < 8 {
                    i as f32 * 40. - 140.
                } else {
                    10000. + i as f32 * 128.
                };
                [x, 0., -10., 0., 1., 1., 0., 0.7, 0.8, 0.9]
            })
            .collect();
        for casts in [false, true] {
            let mut indexed = Standing::new(&placed, &sizes, 1);
            let mut brute = Standing::new(&placed, &sizes, 1);
            for remove in [false, true, false] {
                indexed.restand(&sizes, |i, _| remove && i % 3 == 0);
                brute.restand(&sizes, |i, _| remove && i % 3 == 0);
                indexed.pop.casts = casts;
                brute.pop.casts = casts;
                brute.pop.chunk_index = BoundsIndex::new(
                    brute
                        .pop
                        .chunks
                        .iter()
                        .map(|_| [-1e12, -1e12, -1e12, 1e12, 1e12, 1e12]),
                );
                for x in [-150., 0., 150.] {
                    let camera = Camera {
                        eye: [x, -30., 10.],
                        target: [x, 100., -10.],
                        up: [0., 0., 1.],
                        fov_y: 0.8,
                        near: 1.,
                        size: [1280, 720],
                    };
                    let fixture: serde_json::Value = serde_json::from_str(include_str!(
                        "../../../../tests/fixtures/shadow-vectors.json"
                    ))
                    .unwrap();
                    let mut light: crate::light::LightSettings =
                        serde_json::from_value(fixture["lights"]["game"].clone()).unwrap();
                    light.cascades.max_far_m = 100.;
                    let terrain = crate::static_world::ground::partition::TerrainPartition::new(
                        &crate::static_world::ground::mesh::terrain(
                            &[
                                -300., -100., -10., 300., -100., -10., 300., 100., -10., -300.,
                                100., -10.,
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
                        indexed.pop.select(&view, shadow, &[100., 30., 10.]);
                        brute.pop.select(&view, shadow, &[100., 30., 10.]);
                        assert_eq!(indexed.pop.view, brute.pop.view);
                        assert_eq!(indexed.pop.cast, brute.pop.cast);
                        assert_eq!(indexed.pop.prints, brute.pop.prints);
                        let output = (
                            indexed.pop.view.clone(),
                            indexed.pop.cast.clone(),
                            indexed.pop.prints.clone(),
                        );
                        if remote == 64 {
                            baseline.push(output);
                        } else {
                            assert_eq!(
                                output, baseline[sample],
                                "remote growth changed local output"
                            );
                        }
                        sample += 1;
                        assert!(indexed.pop.visited < 100, "visited {}", indexed.pop.visited);
                    }
                }
            }
        }
    }
}
