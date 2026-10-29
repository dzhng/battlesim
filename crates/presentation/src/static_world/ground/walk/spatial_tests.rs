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

#[test]
fn standing_caster_lists_keep_only_each_cascades_chunks() {
    let sizes = [Size {
        height: 10.,
        radius: 2.,
    }];
    let placed: Vec<f32> = [-200., 0., 200., 1000.]
        .into_iter()
        .flat_map(|x| [x, 0., 0., 0., 1., 1., 0., 0.7, 0.8, 0.9])
        .collect();
    let mut standing = Standing::new(&placed, &sizes, 1);
    let camera = Camera {
        eye: [0., -30., 10.],
        target: [0., 100., 0.],
        up: [0., 0., 1.],
        fov_y: 0.8,
        near: 1.,
        size: [640, 480],
    };
    let view = View::new(&camera);
    let casters = [-200., 0., -100., 5000.].map(|x| {
        let mut m = crate::shadow::IDENTITY;
        m[0] = 1. / 110.;
        m[5] = 1. / 110.;
        m[10] = 1. / 100.;
        m[12] = -x / 110.;
        crate::shadow::camera_record(&m, [0.; 3], 0.1, [64.; 2], 0.)
    });
    standing.pop.select(
        &view,
        Some(&CasterVolumes::new(&casters)),
        &[100., 30., 10.],
    );
    let mut xs: [Vec<f32>; 4] = Default::default();
    standing.pop.each_list(&[0], &mut |l| {
        if l.pass == CAST_PASS {
            xs[l.cascade_mask.trailing_zeros() as usize].extend(l.records.iter().map(|r| r[0]));
        }
    });
    for list in &mut xs {
        list.sort_by(f32::total_cmp);
    }
    assert_eq!(xs, [vec![-200.], vec![0.], vec![-200., 0.], vec![]]);
}

fn felled_fixture(remote: usize) -> (Ground, SceneryWalk) {
    use super::super::{biome::Biome, tables::Grid, Source};
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../../fixtures/presentation.json")).unwrap();
    let mut ground = Ground::new(
        Source {
            map: [0., 0., 32., 32.],
            grid: Grid {
                nx: 2,
                ny: 2,
                spacing: 32.,
                page_size: 16,
                ..Grid::default()
            },
            ..Source::default()
        },
        Biome::load(super::super::biome::tests::SUMMER, None).unwrap(),
        serde_json::from_value(game.clone()).unwrap(),
    )
    .unwrap();
    ground.biome.trees.felled = super::tests::rules();
    let sizes = vec![
        Size {
            height: 11.,
            radius: 4.2
        };
        ground.scenery_kinds().len()
    ];
    let mut scenery = ground.scenery(&sizes);
    scenery.forest = (0..remote + 3)
        .flat_map(|i| {
            let x = match i {
                0 => 0.,
                1 => -200.,
                2 => 10.,
                _ => 10000. + i as f32 * 100.,
            };
            [x, 0., 0., 0.3, 1., 1., 0., 0.7, 0.8, 0.9]
        })
        .collect();
    scenery.forest_ids = (0..remote as u32 + 3).collect();
    scenery.backdrop.clear();
    scenery.understorey.clear();
    let trunks = vec![
        Trunk {
            radius: 0.45,
            crown: 4.
        };
        sizes.len()
    ];
    let mut walk = SceneryWalk::new(scenery, sizes, trunks);
    walk.rows = DRAWS
        .iter()
        .enumerate()
        .map(|(d, _)| {
            (0..walk.sizes.len())
                .map(|k| (d * walk.sizes.len() + k) as u32)
                .collect()
        })
        .collect();
    walk.set_quality(Quality {
        dressing: false,
        understorey: false,
        ..Quality::default()
    });
    // Deliberately non-spatial source order; unknown prop must not become a tree.
    let known = [2, 1, 0]
        .into_iter()
        .chain(3..remote as u32 + 3)
        .chain([u32::MAX])
        .map(|prop| FelledTree {
            prop,
            toward: [1., 0.],
            fell_at: 0.,
        })
        .collect();
    walk.set_felled(known);
    (ground, walk)
}

fn felled_camera(x: f64) -> View {
    View::new(&Camera {
        eye: [x, -30., 6.],
        target: [x, 100., 6.],
        up: [0., 0., 1.],
        fov_y: 0.8,
        near: 1.,
        size: [1280, 720],
    })
}

fn felled_shadows() -> CasterVolumes {
    CasterVolumes::new(&[-200., 0., -100., -5000.].map(|x| {
        let mut m = crate::shadow::IDENTITY;
        m[0] = 1. / 110.;
        m[5] = 1. / 110.;
        m[10] = 1. / 100.;
        m[12] = -x / 110.;
        crate::shadow::camera_record(&m, [0.; 3], 0.1, [64.; 2], 0.)
    }))
}

type FelledOutput = Vec<(u32, u32, u32, u32, u64, Vec<Record>)>;
fn felled_output(walk: &SceneryWalk) -> FelledOutput {
    let mut out = vec![];
    walk.each_list(&mut |l| {
        if l.row >= walk.rows[Draws::Felled as usize - 1][0] && !l.records.is_empty() {
            out.push((
                l.row,
                l.tier,
                l.pass,
                l.cascade_mask,
                l.print,
                l.records.to_vec(),
            ));
        }
    });
    out
}

#[test]
fn felled_remote_growth_preserves_complete_records_and_bounded_work() {
    let mut baseline = vec![];
    for remote in [64, 1024, 4096] {
        let (ground, mut walk) = felled_fixture(remote);
        let shadow = felled_shadows();
        for (step, age) in [0., 1., 2.2, 3.].into_iter().enumerate() {
            walk.set_clock(age);
            walk.select(&ground, &felled_camera(0.), Some(&shadow));
            let output = felled_output(&walk);
            assert!(!output.is_empty());
            if remote == 64 {
                baseline.push(output);
            } else {
                assert_eq!(output, baseline[step]);
            }
            eprintln!("remote={remote} age={age} visited={}", walk.stats()[4]);
            assert!(walk.stats()[4] < 250, "bounds visited: {:?}", walk.stats());
        }
    }
}

#[test]
fn felled_and_stumps_keep_independent_camera_and_cascade_membership() {
    let (ground, mut walk) = felled_fixture(0);
    let view = felled_camera(0.);
    walk.set_clock(1.);
    walk.select(&ground, &view, Some(&felled_shadows()));
    let mut trees: [Vec<f32>; 5] = Default::default();
    let mut stumps: [Vec<f32>; 5] = Default::default();
    let felled_row = walk.rows[Draws::Felled as usize - 1][0];
    let stump_row = walk.rows[Draws::Stump as usize - 1][0];
    walk.each_list(&mut |l| {
        if l.records.is_empty() || (l.row != felled_row && l.row != stump_row) {
            return;
        }
        let pass = if l.pass == VIEW_PASS {
            assert_eq!(l.cascade_mask, 0);
            0
        } else {
            assert_eq!(l.cascade_mask.count_ones(), 1);
            1 + l.cascade_mask.trailing_zeros() as usize
        };
        if l.row == felled_row {
            assert_eq!(l.records.len() % 2, 0);
            for pair in l.records.as_chunks::<2>().0 {
                trees[pass].push(pair[0][0]);
                let p = walk
                    .scenery
                    .forest
                    .as_chunks::<FLOATS>()
                    .0
                    .iter()
                    .find(|p| p[0] == pair[0][0])
                    .unwrap();
                let (mut fall, kind, h, reach) = instance(p, &walk.sizes);
                let t = walk.trunks[kind];
                let pose = super::felled::fell_pose(
                    &ground.biome.trees.felled,
                    1.,
                    h as f64,
                    reach as f64,
                    t.radius,
                    t.crown,
                );
                fall[4..8].copy_from_slice(&[1., 0., pose[0] as f32, pose[1] as f32]);
                let mut rest = [0.; 16];
                for (r, v) in rest.iter_mut().zip(&pose[2..]) {
                    *r = *v as f32;
                }
                assert_eq!(*pair, [fall, rest]);
            }
        } else {
            stumps[pass].extend(l.records.iter().map(|r| r[0]));
        }
    });
    assert_eq!(
        trees,
        [
            vec![10., 0.],
            vec![-200.],
            vec![10., 0.],
            vec![10., 0., -200.],
            vec![]
        ]
    );
    assert_eq!(
        stumps,
        [
            vec![10., 0.],
            vec![-200.],
            vec![10., 0.],
            vec![10., -200., 0.],
            vec![]
        ]
    );
    walk.select(&ground, &view, None);
    assert!(felled_output(&walk).iter().all(|l| l.2 == VIEW_PASS));
    walk.set_felled(vec![FelledTree {
        prop: 1,
        toward: [0., 1.],
        fell_at: 0.,
    }]);
    walk.select(&ground, &view, Some(&felled_shadows()));
    assert!(felled_output(&walk).iter().all(|l| l.2 == CAST_PASS));
    walk.set_felled(vec![]);
    walk.select(&ground, &view, Some(&felled_shadows()));
    assert!(felled_output(&walk).is_empty());
}

#[test]
fn offscreen_falls_age_to_settled_and_reenter_without_restarting() {
    let (ground, mut walk) = felled_fixture(64);
    let away = felled_camera(-5000.);
    walk.select(&ground, &away, None);
    assert!(felled_output(&walk).is_empty());
    assert!(walk.falling);
    for age in [1., 2., 2.49] {
        assert!(walk.set_clock(age));
        walk.select(&ground, &away, None);
        assert!(walk.falling);
        assert!(felled_output(&walk).is_empty());
    }
    assert!(walk.set_clock(2.5));
    walk.select(&ground, &away, None);
    assert!(!walk.falling);
    assert!(!walk.set_clock(10.));
    walk.select(&ground, &felled_camera(0.), None);
    let output = felled_output(&walk);
    assert!(!output.is_empty());
    walk.set_clock(100.);
    walk.select(&ground, &felled_camera(0.), None);
    assert_eq!(
        output,
        felled_output(&walk),
        "settled records and prints stay stable"
    );
    assert!(!walk.set_felled(walk.felled.clone()));
    walk.set_felled(vec![FelledTree {
        prop: u32::MAX,
        toward: [1., 0.],
        fell_at: 1000.,
    }]);
    walk.select(&ground, &away, None);
    assert!(
        !walk.falling,
        "unknown props cannot keep the animation alive"
    );
}

#[test]
fn fixed_fall_envelope_keeps_tipped_compressed_and_stump_extremes() {
    for (sxy, sz) in [(1., 1.), (3., 0.02), (0.2, 4.)] {
        let (ground, initial) = felled_fixture(0);
        let mut scenery = initial.scenery;
        scenery.forest.truncate(FLOATS);
        scenery.forest_ids.truncate(1);
        scenery.forest[4] = sxy;
        scenery.forest[5] = sz;
        let mut walk = SceneryWalk::new(scenery, initial.sizes, initial.trunks);
        walk.rows = initial.rows;
        walk.set_felled(vec![FelledTree {
            prop: 0,
            toward: [0.6, 0.8],
            fell_at: 0.,
        }]);
        let rules = &ground.biome.trees.felled;
        let h = 11. * sz as f64;
        let radius = 4.2 * sxy as f64;
        let trunk = 0.45 * sxy as f64;
        for age in [0., 0.8, 1.8, 2., 2.25, 3., 100.] {
            walk.set_clock(age);
            let pose = super::felled::fell_pose(rules, age, h, radius, trunk, 4. * sz as f64);
            let [angle, hinge, squash, spread, _, landed, _] = pose;
            // Extremal crown vertices in fall coordinates, as transformed by the
            // shader: compressed radial components, hinge and two-radius kick.
            let mut points = vec![[radius, 0., hinge], [-radius, 0., 0.]];
            for along in [-radius, radius] {
                for across in [-radius, radius] {
                    for z in [0., (h - hinge).max(0.)] {
                        let a =
                            along * libm::cos(angle) + z * libm::sin(angle) + 2. * trunk * landed;
                        let b = across * spread;
                        points.push([
                            0.6 * a - 0.8 * b,
                            0.8 * a + 0.6 * b,
                            trunk * landed + hinge * (1. - landed) + z * libm::cos(angle)
                                - along * squash * libm::sin(angle),
                        ]);
                    }
                }
            }
            for point in points {
                let mut m = crate::shadow::IDENTITY;
                m[12] = -point[0] as f32;
                m[13] = -point[1] as f32;
                m[14] = 0.5 - point[2] as f32;
                let shadow = CasterVolumes::new(&[crate::shadow::camera_record(
                    &m, [0.; 3], 0.1, [64.; 2], 0.,
                )]);
                walk.select_felled(
                    &ground,
                    &felled_camera(-5000.),
                    Some(&shadow),
                    &[150., 60., 24.],
                );
                let out = felled_output(&walk);
                assert!(
                    out.iter()
                        .any(|l| l.0 == walk.rows[4][0] && l.2 == CAST_PASS && l.3 == 1),
                    "lost point {point:?}, scale {sxy}/{sz}, age {age}"
                );
                assert!(out
                    .iter()
                    .any(|l| l.0 == walk.rows[5][0] && l.2 == CAST_PASS && l.3 == 1));
            }
        }
    }
}

#[test]
fn one_remote_fall_does_not_rebuild_standing_geometry() {
    let mut local_work = None;
    for remote in [64, 1024, 4096] {
        let (ground, mut walk) = felled_fixture(remote);
        let mut known = walk.felled.clone();
        known.retain(|f| f.prop != 0 && f.prop != remote as u32 + 2);
        walk.set_felled(known.clone());
        walk.set_clock(10.);
        walk.select(&ground, &felled_camera(0.), Some(&felled_shadows()));
        let sources_bytes: usize = [&walk.forest, &walk.understorey.pop, &walk.backdrop]
            .iter()
            .map(|p| {
                p.sources.capacity() * std::mem::size_of::<Vec<usize>>()
                    + p.sources
                        .iter()
                        .map(|s| s.capacity() * std::mem::size_of::<usize>())
                        .sum::<usize>()
            })
            .sum();
        eprintln!("N={} source_vectors_bytes={sources_bytes} removed_placed_bytes={} known_usable_capacity={} chunks={}", remote+3, walk.scenery.forest.len()*4, walk.felled_spatial.known.capacity(), walk.forest.chunks.len());
        let p = &walk.forest;
        let geometry_bytes = p.sorted.capacity() * std::mem::size_of::<Vec<Record>>()
            + p.heights.capacity() * std::mem::size_of::<Vec<f32>>()
            + p.sources.capacity() * std::mem::size_of::<Vec<usize>>()
            + p.sorted
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<Record>())
                .sum::<usize>()
            + p.heights.iter().map(|v| v.capacity() * 4).sum::<usize>()
            + p.sources
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<usize>())
                .sum::<usize>()
            + p.chunks.capacity() * std::mem::size_of::<Chunk>()
            + p.chunks
                .iter()
                .map(|c| (c.start.capacity() + c.end.capacity()) * std::mem::size_of::<usize>())
                .sum::<usize>();
        eprintln!("retained_forest_geometry_bytes={geometry_bytes} (vectors/chunks only; excludes index, snapshot and hash table)");
        let before = walk.forest.generation;
        let work = walk.felled_work();
        let mut records = vec![];
        walk.each_list(&mut |l| {
            if !l.records.is_empty() {
                records.push((
                    l.row,
                    l.tier,
                    l.pass,
                    l.cascade_mask,
                    l.print,
                    l.records.to_vec(),
                ));
            }
        });
        let start = std::time::Instant::now();
        known.push(FelledTree {
            prop: remote as u32 + 2,
            toward: [0., 1.],
            fell_at: 10.,
        });
        let snapshot_rows = known.len();
        walk.set_felled(known);
        let reconcile = start.elapsed();
        walk.select(&ground, &felled_camera(0.), Some(&felled_shadows()));
        eprintln!(
            "remote={remote} reconcile={reconcile:?} total={:?} work={:?} snapshot_delta={}",
            start.elapsed(),
            walk.felled_work(),
            walk.felled_work()[1] - work[1]
        );
        assert_eq!(
            work[0],
            remote + 3,
            "initialize each immutable tree envelope once"
        );
        assert_eq!(walk.felled_work()[1] - work[1], snapshot_rows);
        assert_eq!(work[2], *local_work.get_or_insert(work[2]));
        assert_eq!(
            walk.felled_work()[0],
            work[0],
            "remote fall rebuilt motion envelopes"
        );
        assert_eq!(
            walk.felled_work()[2],
            work[2],
            "remote fall increased local membership work"
        );
        assert_eq!(
            walk.forest.generation, before,
            "a remote fall rebuilt the forest"
        );
        let mut after = vec![];
        walk.each_list(&mut |l| {
            if !l.records.is_empty() {
                after.push((
                    l.row,
                    l.tier,
                    l.pass,
                    l.cascade_mask,
                    l.print,
                    l.records.to_vec(),
                ));
            }
        });
        assert_eq!(
            records, after,
            "remote knowledge changed local lists or prints"
        );
    }
}

#[test]
fn local_fall_filters_standing_records_and_restoring_knowledge_restores_prints() {
    let (ground, mut walk) = felled_fixture(64);
    walk.set_felled(vec![]);
    let shadow = felled_shadows();
    for x in [0., -150., 100.] {
        let view = felled_camera(x);
        walk.select(&ground, &view, Some(&shadow));
        let before = (
            walk.forest.view.clone(),
            walk.forest.cast.clone(),
            walk.forest.prints.clone(),
        );
        walk.set_felled(vec![FelledTree {
            prop: 2,
            toward: [1., 0.],
            fell_at: 0.,
        }]);
        walk.select(&ground, &view, Some(&shadow));
        for (expected, actual) in before
            .0
            .iter()
            .chain(&before.1)
            .zip(walk.forest.view.iter().chain(&walk.forest.cast))
        {
            let kept: Vec<_> = expected.iter().copied().filter(|r| r[0] != 10.).collect();
            assert_eq!(*actual, kept, "standing survivor records/order");
        }
        walk.set_felled(vec![]);
        walk.select(&ground, &view, Some(&shadow));
        assert_eq!(
            before,
            (
                walk.forest.view.clone(),
                walk.forest.cast.clone(),
                walk.forest.prints.clone()
            )
        );
    }
}

/// A crown bends past where its tree stands: a tree whose rest box lies
/// just outside a cascade, by less than the farthest the wind can carry it
/// (`SWAY.reach`), still casts there, and one past that reach does not.
#[test]
fn a_standing_tree_casts_wherever_the_wind_can_carry_its_crown() {
    use crate::static_world::ground::sway::SWAY;
    let (height, radius) = (10., 2.);
    let reach = SWAY.reach(height) as f32;
    assert!(reach > 0.5, "the test needs a reach to probe: {reach}");
    let sizes = [Size { height, radius }];
    let view = View::new(&Camera {
        eye: [0., -30., 10.],
        target: [0., 100., 0.],
        up: [0., 0., 1.],
        fov_y: 0.3,
        near: 1.,
        size: [640, 480],
    });
    // The cascade covers x from -92 to 128, the edge of a 128 m chunk; each
    // tree stands alone past it, its rest box its foot plus its radius.
    let mut m = crate::shadow::IDENTITY;
    m[0] = 1. / 110.;
    m[5] = 1. / 110.;
    m[10] = 1. / 100.;
    m[12] = -18. / 110.;
    let caster = crate::shadow::camera_record(&m, [0.; 3], 0.1, [64.; 2], 0.);
    let casts = |x: f32| {
        let placed = [x, 0., 0., 0., 1., 1., 0., 0.7, 0.8, 0.9];
        let mut standing = Standing::new(&placed, &sizes, 1);
        standing.pop.select(
            &view,
            Some(&CasterVolumes::new(&[caster])),
            &[100., 30., 10.],
        );
        let mut cast = false;
        standing.pop.each_list(&[0], &mut |l| {
            cast |= l.pass == CAST_PASS && !l.records.is_empty();
        });
        cast
    };
    let edge = 128. + radius as f32;
    assert!(casts(edge + 0.5 * reach), "a crown the wind carries in");
    assert!(
        !casts(edge + reach + 1.),
        "a crown the wind cannot carry in"
    );
}
